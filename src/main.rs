// src/main.rs
use std::env;
use std::path::Path;
use std::process;

// Define los módulos localmente
mod epub;
mod render;
mod navigation;
mod metadata;
mod errors;
mod ui;

use epub::EpubDocument;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Uso: {} <ruta_al_archivo.epub>", args[0]);
        process::exit(1);
    }

    let epub_path = Path::new(&args[1]);
    if !epub_path.exists() || epub_path.extension().map_or(true, |ext| ext != "epub") {
        eprintln!("Error: El archivo '{}' no existe o no es un archivo .epub", args[1]);
        process::exit(1);
    }

    // Abrir y parsear el EPUB
    let mut epub_doc = match EpubDocument::open(epub_path) {
        Ok(doc) => doc,
        Err(e) => {
            eprintln!("Error al abrir o parsear el EPUB: {}", e);
            process::exit(1);
        }
    };

    // Verificar que el EPUB tenga capítulos
    let navigator = epub_doc.create_navigator();
    if navigator.total_chapters() == 0 {
        eprintln!("Error: El EPUB no contiene capítulos en el spine o no se pudo leer.");
        process::exit(1);
    }

    // Iniciar la interfaz de usuario con ratatui
    if let Err(e) = ui::start_ui(&mut epub_doc) {
        eprintln!("Error al iniciar la interfaz de usuario: {}", e);
        process::exit(1);
    }
}
