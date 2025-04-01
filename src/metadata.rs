// src/metadata.rs
use roxmltree::Node;
use crate::errors::EpubError;

#[derive(Debug, Default)]
pub struct Metadata {
    pub title: Option<String>,
    pub creator: Option<String>,
    pub language: Option<String>,
    pub identifier: Option<String>,
    pub publisher: Option<String>,
    pub date: Option<String>,
    // Puedes añadir más campos según necesites (subject, description, rights, etc.)
}

impl Metadata {
    // Parsea los metadatos desde el nodo <metadata> del archivo OPF
    pub fn parse(metadata_node: Node) -> Result<Self, EpubError> {
        let mut metadata = Metadata::default();

        for child in metadata_node.children().filter(Node::is_element) {
            // Usamos local_name() para ignorar prefijos de namespace (dc:, etc.)
            match child.tag_name().name() {
                "title" => metadata.title = child.text().map(str::to_string),
                "creator" => metadata.creator = child.text().map(str::to_string),
                "language" => metadata.language = child.text().map(str::to_string),
                "identifier" => metadata.identifier = child.text().map(str::to_string),
                "publisher" => metadata.publisher = child.text().map(str::to_string),
                "date" => metadata.date = child.text().map(str::to_string),
                _ => {} // Ignora otros elementos de metadatos por ahora
            }
        }
        Ok(metadata)
    }
}

// Función para mostrar los metadatos de forma legible
#[allow(dead_code)]
pub fn display_metadata(metadata: &Metadata) {
    println!("--- Metadatos ---");
    println!("Título: {}", metadata.title.as_deref().unwrap_or("N/A"));
    println!("Autor: {}", metadata.creator.as_deref().unwrap_or("N/A"));
    println!("Idioma: {}", metadata.language.as_deref().unwrap_or("N/A"));
    println!("Identificador: {}", metadata.identifier.as_deref().unwrap_or("N/A"));
    println!("Editor: {}", metadata.publisher.as_deref().unwrap_or("N/A"));
    println!("Fecha: {}", metadata.date.as_deref().unwrap_or("N/A"));
    println!("---------------");
}
