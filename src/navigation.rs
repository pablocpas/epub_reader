// src/navigation.rs
use std::collections::HashMap;
use crate::epub::ManifestItem; // Necesitaremos esto más tarde
use crate::errors::EpubError;

// Representa una entrada en la Tabla de Contenidos (TOC)
#[derive(Debug, Clone)]
pub struct TocEntry {
    pub label: String,
    #[allow(dead_code)]
    pub href: String, // Ruta resuelta dentro del EPUB
    #[allow(dead_code)]
    pub id: Option<String>, // ID opcional del navPoint/li
}

// Gestiona el estado de la navegación
#[derive(Debug)]
pub struct Navigator {
    // Items en el orden de lectura definido por <spine>
    spine_ids: Vec<String>,
    // Índice actual dentro de spine_ids
    current_spine_index: usize,
    // Tabla de contenidos para mostrar al usuario (puede no coincidir 1:1 con el spine)
    toc: Vec<TocEntry>,
    // Mapa para buscar rápidamente hrefs desde IDs (del manifiesto)
    manifest: HashMap<String, ManifestItem>,
    // Directorio base para resolver rutas relativas (directorio del OPF)
    root_path: String,
}

impl Navigator {
    pub fn new(
        spine_ids: Vec<String>,
        toc: Vec<TocEntry>,
        manifest: HashMap<String, ManifestItem>,
        root_path: String,
    ) -> Self {
        Navigator {
            spine_ids,
            current_spine_index: 0,
            toc,
            manifest,
            root_path,
        }
    }

    // Avanza al siguiente capítulo en el spine
    pub fn next(&mut self) -> bool {
        if self.current_spine_index + 1 < self.spine_ids.len() {
            self.current_spine_index += 1;
            true
        } else {
            false // Ya está en el último capítulo
        }
    }

    // Retrocede al capítulo anterior en el spine
    pub fn prev(&mut self) -> bool {
        if self.current_spine_index > 0 {
            self.current_spine_index -= 1;
            true
        } else {
            false // Ya está en el primer capítulo
        }
    }

    // Va a un capítulo específico por su índice (basado en 1 para el usuario)
    pub fn goto(&mut self, index_one_based: usize) -> bool {
        if index_one_based > 0 && index_one_based <= self.spine_ids.len() {
            self.current_spine_index = index_one_based - 1;
            true
        } else {
            false // Índice inválido
        }
    }

    // Obtiene el ID del capítulo actual en el spine
    pub fn current_chapter_id(&self) -> Option<&str> {
        self.spine_ids.get(self.current_spine_index).map(String::as_str)
    }

    // Obtiene la ruta (href) del capítulo actual
    pub fn current_chapter_href(&self) -> Result<String, EpubError> {
        let id = self.current_chapter_id()
            .ok_or_else(|| EpubError::InvalidChapterIndex(self.current_spine_index))?;

        let manifest_item = self.manifest.get(id)
            .ok_or_else(|| EpubError::ManifestItemNotFound(id.to_string()))?;

        // Construye la ruta completa dentro del archivo ZIP
        // self.root_path es el directorio que contiene el OPF
        // manifest_item.href es relativo a ese directorio
        let full_path = if self.root_path.is_empty() {
            manifest_item.href.clone()
        } else {
            format!("{}/{}", self.root_path, manifest_item.href)
        };

        Ok(full_path.replace("//", "/")) // Simple normalización
    }


    // Devuelve el número de capítulo actual (basado en 1) y el total
    pub fn current_position(&self) -> (usize, usize) {
        (self.current_spine_index + 1, self.spine_ids.len())
    }

// Muestra la tabla de contenidos (TOC)
#[allow(dead_code)]
pub fn display_toc(&self) {
    println!("--- Tabla de Contenidos ---");
    if self.toc.is_empty() {
        println!(" (No se encontró o no se pudo parsear la tabla de contenidos)");
    } else {
        for (i, entry) in self.toc.iter().enumerate() {
            // Intentamos encontrar el índice del spine que corresponde a este href
            // Esto es una aproximación, ya que TOC y Spine no siempre coinciden perfectamente
            let spine_index = self.spine_ids.iter().position(|id| {
                self.manifest.get(id).map_or(false, |item| {
                   let item_full_path = if self.root_path.is_empty() { item.href.clone() } else { format!("{}/{}", self.root_path, item.href) };
                   item_full_path.replace("//", "/") == entry.href
                })
            });
            if let Some(idx) = spine_index {
                 println!("{:>3}. {} (Ir con: goto {})", i + 1, entry.label, idx + 1);
            } else {
                // Si no está en el spine, no se puede ir directamente con 'goto'
                 println!("{:>3}. {}", i + 1, entry.label);
            }

        }
    }
    println!("---------------------------");
}

    // Devuelve el número total de capítulos en el spine
    pub fn total_chapters(&self) -> usize {
        self.spine_ids.len()
    }
    
    // Devuelve una referencia a la tabla de contenidos
    pub fn get_toc(&self) -> &Vec<TocEntry> {
        &self.toc
    }
}

// Función helper (simplificada) para normalizar rutas (maneja "./", asume separador '/')
// Una librería como `path_clean` o `lexiclean` sería más robusta
#[allow(dead_code)]
fn normalize_path_simple(path_str: &str) -> String {
    let mut components: Vec<&str> = Vec::new();
    for component in path_str.split('/') {
        match component {
            "." | "" => {} // Ignorar componente actual o vacío
            ".." => { components.pop(); } // Subir un nivel
            _ => components.push(component), // Añadir componente normal
        }
    }
    components.join("/")
}
