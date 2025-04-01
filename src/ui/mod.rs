// src/ui/mod.rs
use std::io;
use std::time::Duration;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame, Terminal,
};
use textwrap::fill;
use unicode_width::UnicodeWidthStr;

use crate::epub::EpubDocument;
use crate::navigation::Navigator;
use crate::metadata::Metadata;

// Modos de la aplicación
pub enum AppMode {
    Normal,
    Command,
}

// Estado de la aplicación
pub struct App<'a> {
    pub epub_doc: &'a mut EpubDocument,
    pub navigator: Navigator,
    pub current_content: String,
    pub command_input: String,
    pub mode: AppMode,
    pub status_message: String,
    pub scroll_offset: u16,      // Scroll para el contenido del capítulo
    pub toc_scroll_offset: u16,  // Scroll exclusivo para la tabla de contenidos
    pub should_quit: bool,
    pub show_metadata: bool,
    pub show_toc: bool,
}

impl<'a> App<'a> {
    pub fn new(epub_doc: &'a mut EpubDocument) -> Self {
        let navigator = epub_doc.create_navigator();
        App {
            epub_doc,
            navigator,
            current_content: String::new(),
            command_input: String::new(),
            mode: AppMode::Normal,
            status_message: String::new(),
            scroll_offset: 0,
            toc_scroll_offset: 0,
            should_quit: false,
            show_metadata: false,
            show_toc: false,
        }
    }

    // Carga el contenido del capítulo actual
    pub fn load_current_chapter(&mut self) {
        match self.navigator.current_chapter_href() {
            Ok(href) => {
                match self.epub_doc.read_chapter_content(&href) {
                    Ok(content) => {
                        let rendered_text = crate::render::render_xhtml_to_text(&content);
                        self.current_content = rendered_text;
                        self.scroll_offset = 0; // Resetear el scroll al cambiar de capítulo
                        self.status_message = format!(
                            "Capítulo {} de {}",
                            self.navigator.current_position().0,
                            self.navigator.current_position().1
                        );
                    }
                    Err(e) => {
                        self.current_content = format!("Error al leer el capítulo: {}", e);
                        self.status_message = "Error al cargar el capítulo".to_string();
                    }
                }
            }
            Err(e) => {
                self.current_content = format!("Error al obtener la ruta del capítulo: {}", e);
                self.status_message = "Error al obtener la ruta del capítulo".to_string();
            }
        }
    }

    // Navega al siguiente capítulo
    pub fn next_chapter(&mut self) {
        if self.navigator.next() {
            self.load_current_chapter();
            self.status_message = format!(
                "Capítulo {} de {}",
                self.navigator.current_position().0,
                self.navigator.current_position().1
            );
        } else {
            self.status_message = "Ya estás en el último capítulo".to_string();
        }
    }

    // Navega al capítulo anterior
    pub fn prev_chapter(&mut self) {
        if self.navigator.prev() {
            self.load_current_chapter();
            self.status_message = format!(
                "Capítulo {} de {}",
                self.navigator.current_position().0,
                self.navigator.current_position().1
            );
        } else {
            self.status_message = "Ya estás en el primer capítulo".to_string();
        }
    }

    // Navega a un capítulo específico
    pub fn goto_chapter(&mut self, index: usize) {
        if self.navigator.goto(index) {
            self.load_current_chapter();
            self.status_message = format!(
                "Capítulo {} de {}",
                self.navigator.current_position().0,
                self.navigator.current_position().1
            );
        } else {
            self.status_message = format!("Capítulo {} no válido", index);
        }
    }

    // Procesa la entrada de comandos
    pub fn process_command(&mut self) {
        let cmd = self.command_input.trim().to_lowercase();
        let parts: Vec<&str> = cmd.split_whitespace().collect();

        match parts.as_slice() {
            ["q"] | ["quit"] => {
                self.should_quit = true;
            }
            ["n"] | ["next"] => {
                self.next_chapter();
            }
            ["p"] | ["prev"] => {
                self.prev_chapter();
            }
            ["g", index_str] | ["goto", index_str] => {
                if let Ok(index) = index_str.parse::<usize>() {
                    self.goto_chapter(index);
                } else {
                    self.status_message = format!("Número de capítulo inválido: {}", index_str);
                }
            }
            ["t"] | ["toc"] => {
                self.show_toc = true;
                self.show_metadata = false;
                self.toc_scroll_offset = 0; // Reiniciar scroll de TOC al entrar
            }
            ["m"] | ["meta"] => {
                self.show_metadata = true;
                self.show_toc = false;
            }
            [] => {
                // Comando vacío, no hacer nada
            }
            _ => {
                self.status_message = format!("Comando desconocido: {}", cmd);
            }
        }

        self.command_input.clear();
        self.mode = AppMode::Normal;
    }

    // Maneja eventos de teclado
    pub fn handle_key_event(&mut self, key: KeyCode, modifiers: KeyModifiers) {
        match self.mode {
            AppMode::Normal => {
                if self.show_toc {
                    // Manejo específico para la tabla de contenidos
                    match key {
                        KeyCode::Char('j') => {
                            self.toc_scroll_offset = self.toc_scroll_offset.saturating_add(1);
                        }
                        KeyCode::Char('k') => {
                            self.toc_scroll_offset = self.toc_scroll_offset.saturating_sub(1);
                        }
                        KeyCode::Esc => {
                            self.show_toc = false;
                            self.toc_scroll_offset = 0;
                        }
                        _ => {}
                    }
                } else {
                    // Manejo para el contenido del capítulo
                    match key {
                        KeyCode::Char('j') => {
                            self.scroll_offset = self.scroll_offset.saturating_add(1);
                        }
                        KeyCode::Char('k') => {
                            self.scroll_offset = self.scroll_offset.saturating_sub(1);
                        }
                        KeyCode::Char('d') if modifiers.contains(KeyModifiers::CONTROL) => {
                            self.scroll_offset = self.scroll_offset.saturating_add(10);
                        }
                        KeyCode::Char('u') if modifiers.contains(KeyModifiers::CONTROL) => {
                            self.scroll_offset = self.scroll_offset.saturating_sub(10);
                        }
                        KeyCode::Char('g') if modifiers.contains(KeyModifiers::SHIFT) => {
                            self.scroll_offset = u16::MAX; // Ir al final del texto
                        }
                        KeyCode::Char('g') => {
                            self.scroll_offset = 0; // Ir al inicio del texto
                        }
                        KeyCode::Char('n') => {
                            self.next_chapter();
                        }
                        KeyCode::Char('p') => {
                            self.prev_chapter();
                        }
                        KeyCode::Char(':') => {
                            self.mode = AppMode::Command;
                            self.command_input.clear();
                        }
                        KeyCode::Char('q') => {
                            self.should_quit = true;
                        }
                        KeyCode::Esc => {
                            // Salir de vistas especiales (TOC o metadata)
                            self.show_toc = false;
                            self.show_metadata = false;
                        }
                        _ => {}
                    }
                }
            }
            AppMode::Command => match key {
                KeyCode::Enter => {
                    self.process_command();
                }
                KeyCode::Char(c) => {
                    self.command_input.push(c);
                }
                KeyCode::Backspace => {
                    self.command_input.pop();
                }
                KeyCode::Esc => {
                    self.command_input.clear();
                    self.mode = AppMode::Normal;
                }
                _ => {}
            },
        }
    }
}

// Función para ejecutar la UI
pub fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    // Cargar el primer capítulo
    app.load_current_chapter();

    loop {
        terminal.draw(|f| ui::<B>(f, app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == event::KeyEventKind::Press {
                    app.handle_key_event(key.code, key.modifiers);
                }
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

// Función para renderizar la UI
fn ui<B: Backend>(f: &mut Frame<'_>, app: &App) {
    let size = f.size();

    // Crear el layout principal
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // Barra de estado superior
            Constraint::Min(1),     // Contenido principal
            Constraint::Length(1),  // Barra de estado inferior o entrada de comando
        ])
        .split(size);

    // Renderizar la barra de estado superior
    let title = match app.navigator.current_position() {
        (current, total) => format!("EPUB Reader - Capítulo {} de {}", current, total),
    };
    let title_widget = Paragraph::new(title)
        .style(Style::default().bg(Color::Blue).fg(Color::White));
    f.render_widget(title_widget, chunks[0]);

    // Renderizar el contenido principal
    if app.show_metadata {
        render_metadata::<B>(f, chunks[1], &app.epub_doc.metadata);
    } else if app.show_toc {
        render_toc::<B>(f, chunks[1], app);
    } else {
        render_content::<B>(f, chunks[1], app);
    }

    // Renderizar la barra inferior
    match app.mode {
        AppMode::Normal => {
            let status = Paragraph::new(app.status_message.clone())
                .style(Style::default().bg(Color::Blue).fg(Color::White));
            f.render_widget(status, chunks[2]);
        }
        AppMode::Command => {
            let command = format!(":{}", app.command_input);
            let command_widget = Paragraph::new(command)
                .style(Style::default().bg(Color::Black).fg(Color::White));
            f.render_widget(command_widget, chunks[2]);
        }
    }
}

// Función para renderizar el contenido del capítulo
fn render_content<B: Backend>(f: &mut Frame<'_>, area: Rect, app: &App) {
    // Justificar el texto para que se ajuste al ancho del área
    let width = area.width as usize;
    let justified_text = justify_text(&app.current_content, width);
    
    // Convertir el Text a un vector de Lines para poder modificar el estilo de la línea actual
    let mut lines = justified_text.lines.clone();
    
    // Calcular la altura visible del área de contenido
    let visible_height = area.height as usize;
    
    // Calcular la línea que debe estar en el centro de la pantalla
    let middle_line_idx = visible_height / 2;
    
    // Siempre resaltar la línea del medio de la pantalla visible
    if let Some(middle_line) = lines.get_mut(app.scroll_offset as usize + middle_line_idx) {
        // Resaltar la línea central con un fondo gris oscuro
        let spans = middle_line.spans.clone();
        *middle_line = Line::from(spans).style(Style::default().bg(Color::Rgb(40, 40, 40)));
    }
    
    let highlighted_text = Text::from(lines);

    let text_widget = Paragraph::new(highlighted_text)
        .block(Block::default().borders(Borders::NONE))
        .scroll((app.scroll_offset, 0))
        .wrap(Wrap { trim: true });

    f.render_widget(text_widget, area);
}

// Función para renderizar la tabla de contenidos
fn render_toc<B: Backend>(f: &mut Frame<'_>, area: Rect, app: &App) {
    let mut toc_text = vec![Line::from(vec![
        Span::styled("Tabla de Contenidos", Style::default().add_modifier(Modifier::BOLD))
    ])];

    for (i, entry) in app.navigator.get_toc().iter().enumerate() {
        let line = Line::from(vec![
            Span::raw(format!("{:>3}. ", i + 1)),
            Span::raw(&entry.label),
        ]);
        toc_text.push(line);
    }

    let toc_widget = Paragraph::new(toc_text)
        .block(Block::default().borders(Borders::NONE))
        // Usar el offset específico para la TOC
        .scroll((app.toc_scroll_offset, 0))
        .wrap(Wrap { trim: true });

    f.render_widget(toc_widget, area);
}

// Función para renderizar los metadatos
fn render_metadata<B: Backend>(f: &mut Frame<'_>, area: Rect, metadata: &Metadata) {
    let meta_text = vec![
        Line::from(vec![
            Span::styled("Metadatos", Style::default().add_modifier(Modifier::BOLD))
        ]),
        Line::from(vec![
            Span::raw("Título: "),
            Span::raw(metadata.title.as_deref().unwrap_or("N/A")),
        ]),
        Line::from(vec![
            Span::raw("Autor: "),
            Span::raw(metadata.creator.as_deref().unwrap_or("N/A")),
        ]),
        Line::from(vec![
            Span::raw("Idioma: "),
            Span::raw(metadata.language.as_deref().unwrap_or("N/A")),
        ]),
        Line::from(vec![
            Span::raw("Identificador: "),
            Span::raw(metadata.identifier.as_deref().unwrap_or("N/A")),
        ]),
        Line::from(vec![
            Span::raw("Editor: "),
            Span::raw(metadata.publisher.as_deref().unwrap_or("N/A")),
        ]),
        Line::from(vec![
            Span::raw("Fecha: "),
            Span::raw(metadata.date.as_deref().unwrap_or("N/A")),
        ]),
    ];

    let meta_widget = Paragraph::new(meta_text)
        .block(Block::default().borders(Borders::NONE))
        .wrap(Wrap { trim: true });

    f.render_widget(meta_widget, area);
}

// Función para justificar el texto
fn justify_text(text: &str, width: usize) -> Text {
    let mut justified_lines = Vec::new();
    
    // Primero, envolvemos el texto para que se ajuste al ancho
    let wrapped_text = fill(text, width);
    
    // Luego, procesamos cada línea para justificarla
    for line in wrapped_text.lines() {
        if line.trim().is_empty() {
            justified_lines.push(Line::from(""));
            continue;
        }
        
        // Para títulos y listas, no justificamos
        if line.starts_with('#') || line.starts_with("  -") {
            justified_lines.push(Line::from(line.to_string()));
            continue;
        }
        
        // Justificar la línea si tiene suficiente contenido
        let line_width = UnicodeWidthStr::width(line);
        if line_width > width * 3 / 4 && line_width < width && line.split_whitespace().count() > 1 {
            let words: Vec<&str> = line.split_whitespace().collect();
            let word_count = words.len();
            
            if word_count > 1 {
                let total_word_length: usize = words.iter().map(|w| UnicodeWidthStr::width(*w)).sum();
                let spaces_needed = width - total_word_length;
                let spaces_between = spaces_needed / (word_count - 1);
                let extra_spaces = spaces_needed % (word_count - 1);
                
                let mut justified_line = String::new();
                for (i, word) in words.iter().enumerate() {
                    justified_line.push_str(word);
                    
                    if i < word_count - 1 {
                        let spaces = if i < extra_spaces {
                            spaces_between + 1
                        } else {
                            spaces_between
                        };
                        justified_line.push_str(&" ".repeat(spaces));
                    }
                }
                justified_lines.push(Line::from(justified_line));
            } else {
                justified_lines.push(Line::from(line.to_string()));
            }
        } else {
            justified_lines.push(Line::from(line.to_string()));
        }
    }
    
    Text::from(justified_lines)
}

// Inicializa el terminal y ejecuta la aplicación
pub fn start_ui(epub_doc: &mut EpubDocument) -> io::Result<()> {
    // Configurar el terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Crear la aplicación
    let mut app = App::new(epub_doc);

    // Ejecutar la aplicación
    let res = run_app(&mut terminal, &mut app);

    // Restaurar el terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    res
}
