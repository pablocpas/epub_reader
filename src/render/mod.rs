// src/render/mod.rs
use scraper::{Html, Selector, Node, ElementRef};
use std::fmt::Write; // Para escribir en String

// Parsea el contenido XHTML y lo convierte a texto plano formateado básico
pub fn render_xhtml_to_text(xhtml_content: &str) -> String {
    let document = Html::parse_document(xhtml_content);
    let mut output = String::new();
    // Procesamos el body, o todo el documento si no hay body
    let body_selector = Selector::parse("body").unwrap();
    // Select the body element if it exists, otherwise use the document's root element
    let root_node = document.select(&body_selector).next().unwrap_or_else(|| document.root_element());

    process_node(root_node, &mut output, 0);

    // Limpieza simple: reduce múltiples saltos de línea a un máximo de dos
    let lines: Vec<&str> = output.lines().collect();
    let mut cleaned_output = String::new();
    let mut consecutive_empty_lines = 0;
    for line in lines {
        let trimmed_line = line.trim();
        if trimmed_line.is_empty() {
            consecutive_empty_lines += 1;
            if consecutive_empty_lines <= 2 {
                writeln!(cleaned_output).ok();
            }
        } else {
            consecutive_empty_lines = 0;
            writeln!(cleaned_output, "{}", line).ok(); // Preserva sangría si existe
        }
    }

    cleaned_output.trim().to_string() // Elimina espacios/saltos al inicio/final
}

// Función recursiva para procesar nodos HTML
fn process_node(node: ElementRef, output: &mut String, depth: usize) {
    for child in node.children() {
        match child.value() {
            Node::Text(text) => {
                // Reemplaza múltiples espacios/saltos de línea dentro del texto con uno solo
                let cleaned_text = text.text.split_whitespace().collect::<Vec<_>>().join(" ");
                if !cleaned_text.is_empty() {
                    write!(output, "{}", cleaned_text).ok();
                }
            }
            Node::Element(element) => {
                let tag_name = element.name().to_lowercase();
                let needs_leading_newline = matches!(tag_name.as_str(), "p" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "li" | "div" | "br");
                let needs_trailing_newline = matches!(tag_name.as_str(), "p" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "div" | "br");
                let is_block = needs_leading_newline || needs_trailing_newline;

                // Añadir salto de línea antes de elementos de bloque si no estamos al principio
                if needs_leading_newline && !output.is_empty() && !output.ends_with('\n') {
                    writeln!(output).ok();
                }

                // Procesamiento específico por etiqueta
                match tag_name.as_str() {
                    "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                        write!(output, "# ").ok(); // Estilo Markdown simple
                        if let Some(element_ref) = ElementRef::wrap(child) {
                            process_node(element_ref, output, depth + 1);
                        }
                        writeln!(output).ok(); // Salto de línea extra después de encabezado
                    }
                    "p" => {
                        if let Some(element_ref) = ElementRef::wrap(child) {
                            process_node(element_ref, output, depth + 1);
                        }
                    }
                    "li" => {
                        write!(output, "  - ").ok(); // Sangría y guion para listas
                        if let Some(element_ref) = ElementRef::wrap(child) {
                            process_node(element_ref, output, depth + 1);
                        }
                    }
                    "em" | "i" => {
                        write!(output, "*").ok(); // Cursiva
                        if let Some(element_ref) = ElementRef::wrap(child) {
                            process_node(element_ref, output, depth + 1);
                        }
                        write!(output, "*").ok();
                    }
                    "strong" | "b" => {
                        write!(output, "**").ok(); // Negrita
                        if let Some(element_ref) = ElementRef::wrap(child) {
                            process_node(element_ref, output, depth + 1);
                        }
                        write!(output, "**").ok();
                    }
                    "br" => {
                        // Ya manejado por needs_leading/trailing_newline
                    }
                    "img" | "script" | "style" | "link" | "head" | "meta" => {
                        // Ignorar estos elementos y su contenido
                    }
                    // Para otros elementos (div, span, etc.), procesa hijos directamente
                    _ => {
                        if let Some(element_ref) = ElementRef::wrap(child) {
                            process_node(element_ref, output, depth + 1);
                        }
                    }
                }

                // Añadir salto de línea después de elementos de bloque
                if needs_trailing_newline {
                    // Asegúrate de que no haya ya un salto de línea
                    if !output.ends_with('\n') {
                        writeln!(output).ok();
                    }
                    // Añadir un salto extra después de párrafos para mejor separación
                    if tag_name == "p" && !output.ends_with("\n\n") {
                        writeln!(output).ok();
                    }
                } else if is_block && !tag_name.is_empty() {
                    // Añade un espacio después de elementos en línea si no son seguidos por puntuación o espacio
                    if !output.ends_with(char::is_whitespace) && !output.ends_with(|c: char| c.is_ascii_punctuation()) {
                        write!(output," ").ok();
                    }
                }
            }
            _ => {
                // Ignorar comentarios, Doctype, etc.
            }
        }
    }
}
