
// src/epub/mod.rs
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, BufReader};
use std::path::{Path, PathBuf};
use zip::ZipArchive;
use roxmltree::{Document, Node};

use crate::metadata::Metadata;
use crate::navigation::{Navigator, TocEntry};
use crate::errors::EpubError;

const CONTAINER_PATH: &str = "META-INF/container.xml";
const OPF_MIME_TYPE: &str = "application/oebps-package+xml";

// Representa un item en el manifiesto del OPF
#[derive(Debug, Clone)]
pub struct ManifestItem {
    #[allow(dead_code)]
    pub id: String,
    pub href: String,
    #[allow(dead_code)]
    pub media_type: String,
    pub properties: Option<String>, // Para identificar el archivo NAV en EPUB3
}

// Estructura principal que contiene la información parseada del EPUB
#[derive(Debug)]
pub struct EpubDocument {
    // Mantenemos el archivo abierto para leer contenido bajo demanda
    // Nota: Esto significa que el archivo EPUB no debe ser movido/eliminado
    // mientras el programa se ejecuta. Una alternativa es leer todo en memoria
    // o reabrir el archivo cada vez (menos eficiente).
    // Usamos BufReader para mejorar eficiencia de lectura.
    archive: ZipArchive<BufReader<File>>,
    pub metadata: Metadata,
    pub manifest: HashMap<String, ManifestItem>,
    pub spine_ids: Vec<String>, // IDs de los items del spine en orden
    pub toc: Vec<TocEntry>,
    #[allow(dead_code)]
    opf_path: PathBuf, // Ruta del archivo OPF dentro del ZIP
    root_path: String, // Directorio que contiene el OPF (para resolver rutas relativas)
}

impl EpubDocument {
    // Función principal para abrir y parsear un archivo EPUB
    pub fn open(path: &Path) -> Result<Self, EpubError> {
        let file = File::open(path)?;
        let buf_reader = BufReader::new(file); // Envuelve File en BufReader
        let mut archive = ZipArchive::new(buf_reader)?;

        // 1. Parsear container.xml para encontrar el archivo OPF
        let opf_path_str = parse_container(&mut archive)?;
        let opf_path = PathBuf::from(&opf_path_str);

        // Determinar el directorio raíz (el que contiene el OPF)
        let root_path = opf_path.parent()
            .and_then(|p| p.to_str())
            .unwrap_or("")
            .to_string();

        // 2. Leer y parsear el archivo OPF
        let opf_content = read_entry_to_string(&mut archive, &opf_path_str)?;
        let opf_doc = Document::parse(&opf_content)?;

        let package_node = if opf_doc.root_element().tag_name().name() == "package" {
            opf_doc.root_element()
        } else {
            opf_doc.root_element()
                .children().find(|n| n.tag_name().name() == "package")
                .ok_or(EpubError::MissingPackageElement)?
        };

        // 3. Parsear Metadatos
        let metadata_node = package_node.children().find(|n| n.tag_name().name() == "metadata")
            .ok_or(EpubError::MissingMetadataElement)?;
        let metadata = Metadata::parse(metadata_node)?;

        // 4. Parsear Manifiesto
        let manifest_node = package_node.children().find(|n| n.tag_name().name() == "manifest")
            .ok_or(EpubError::MissingManifestElement)?;
        let manifest = parse_manifest(manifest_node)?;

        // 5. Parsear Spine
        let spine_node = package_node.children().find(|n| n.tag_name().name() == "spine")
            .ok_or(EpubError::MissingSpineElement)?;
        let spine_ids = parse_spine(spine_node)?;

        // 6. Encontrar y parsear la Tabla de Contenidos (TOC)
        let toc = parse_toc(&mut archive, &manifest, &root_path, spine_node)?;

        Ok(EpubDocument {
            archive,
            metadata,
            manifest,
            spine_ids,
            toc,
            opf_path,
            root_path,
        })
    }

    // Lee el contenido de un capítulo (archivo XHTML) por su ID del spine
    // Mut borrow of self.archive needed here.
    pub fn read_chapter_content(&mut self, href: &str) -> Result<String, EpubError> {
        // El href ya debería ser la ruta completa dentro del zip
        read_entry_to_string(&mut self.archive, href)
            .map_err(|e| match e {
                // Proporciona un contexto más específico si falla la lectura
                EpubError::Zip(zip::result::ZipError::FileNotFound) => EpubError::ContentReadError(format!("Archivo no encontrado en el ZIP: {}", href)),
                other_err => other_err,
            })
    }

    // Crea el navegador
     pub fn create_navigator(&self) -> Navigator {
        Navigator::new(
            self.spine_ids.clone(),
            self.toc.clone(),
            self.manifest.clone(),
            self.root_path.clone(),
        )
    }
}


// --- Funciones auxiliares de parsing ---

fn read_entry_to_string<R: Read + std::io::Seek>(archive: &mut ZipArchive<R>, path: &str) -> Result<String, EpubError> {
    let mut entry = archive.by_name(path)?;
    let mut content = String::new();
    entry.read_to_string(&mut content)?;
    Ok(content)
}


fn parse_container<R: Read + std::io::Seek>(archive: &mut ZipArchive<R>) -> Result<String, EpubError> {
    let container_content = read_entry_to_string(archive, CONTAINER_PATH)
        .map_err(|_| EpubError::MissingContainerXml)?; // Error específico si container.xml falta

    let doc = Document::parse(&container_content)?;
    let rootfile_node = doc.descendants()
        .find(|n| n.tag_name().name() == "rootfile")
        .ok_or(EpubError::MissingRootfileElement)?;

    let opf_path = rootfile_node.attribute("full-path")
        .ok_or(EpubError::MissingFullPathAttribute)?;

    // Validar que el tipo sea el esperado (opcional pero bueno)
    let media_type = rootfile_node.attribute("media-type");
    if media_type != Some(OPF_MIME_TYPE) {
        // Podrías loggear un warning aquí si quieres
        eprintln!("Advertencia: media-type del rootfile no es '{}', es {:?}. Continuando...", OPF_MIME_TYPE, media_type);
    }

    Ok(opf_path.to_string())
}

fn parse_manifest(manifest_node: Node) -> Result<HashMap<String, ManifestItem>, EpubError> {
    let mut manifest = HashMap::new();
    for item_node in manifest_node.children().filter(|n| n.tag_name().name() == "item") {
        let id = item_node.attribute("id").ok_or_else(|| EpubError::XmlTextExtractionError)?.to_string();
        let href = item_node.attribute("href").ok_or_else(|| EpubError::XmlTextExtractionError)?.to_string();
        let media_type = item_node.attribute("media-type").ok_or_else(|| EpubError::XmlTextExtractionError)?.to_string();
        let properties = item_node.attribute("properties").map(str::to_string);

        manifest.insert(id.clone(), ManifestItem { id, href, media_type, properties });
    }
    Ok(manifest)
}

fn parse_spine(spine_node: Node) -> Result<Vec<String>, EpubError> {
    let mut spine_ids = Vec::new();
    for itemref_node in spine_node.children().filter(|n| n.tag_name().name() == "itemref") {
        let idref = itemref_node.attribute("idref").ok_or_else(|| EpubError::XmlTextExtractionError)?.to_string();
        spine_ids.push(idref);
    }
    Ok(spine_ids)
}

fn parse_toc<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
    manifest: &HashMap<String, ManifestItem>,
    root_path: &str,
    spine_node: Node, // Necesario para buscar el ID del toc.ncx
) -> Result<Vec<TocEntry>, EpubError> {
    // Estrategia:
    // 1. Buscar en el manifiesto un item con properties="nav" (EPUB 3).
    // 2. Si no se encuentra, buscar el ID del toc.ncx en el atributo 'toc' del <spine>.
    // 3. Si se encuentra, buscar ese ID en el manifiesto para obtener el href.
    // 4. Parsear el archivo encontrado (nav.xhtml o toc.ncx).

    // Buscar Nav XHTML (EPUB 3)
    if let Some(nav_item) = manifest.values().find(|item| item.properties.as_deref() == Some("nav")) {
        let nav_href = build_full_path(root_path, &nav_item.href);
        match read_entry_to_string(archive, &nav_href) {
             Ok(nav_content) => {
                 match parse_nav_xhtml(&nav_content, root_path, &nav_href) {
                    Ok(toc) if !toc.is_empty() => return Ok(toc),
                    Ok(_) => eprintln!("Advertencia: Se encontró nav.xhtml pero no contenía entradas de TOC válidas."),
                    Err(e) => eprintln!("Advertencia: Error al parsear nav.xhtml: {}", e),
                 }
             }
             Err(e) => eprintln!("Advertencia: No se pudo leer el archivo nav referenciado: {} ({})", nav_href, e),
        }
    }


    // Buscar toc.ncx (EPUB 2)
    if let Some(toc_id) = spine_node.attribute("toc") {
        if let Some(ncx_item) = manifest.get(toc_id) {
             let ncx_href = build_full_path(root_path, &ncx_item.href);
            match read_entry_to_string(archive, &ncx_href) {
                Ok(ncx_content) => {
                    match parse_ncx(&ncx_content, root_path, &ncx_href) {
                         Ok(toc) if !toc.is_empty() => return Ok(toc),
                         Ok(_) => eprintln!("Advertencia: Se encontró toc.ncx pero no contenía entradas válidas."),
                         Err(e) => eprintln!("Advertencia: Error al parsear toc.ncx: {}", e),
                    }
                }
                 Err(e) => eprintln!("Advertencia: No se pudo leer el archivo NCX referenciado: {} ({})", ncx_href, e),
            }
        } else {
             eprintln!("Advertencia: El ID del TOC '{}' del spine no se encontró en el manifiesto.", toc_id);
        }
    }

    // Si no se encontró ninguno de los dos
     eprintln!("Advertencia: No se pudo encontrar o parsear un archivo de tabla de contenidos (nav.xhtml o toc.ncx). La navegación por TOC no estará disponible.");
     Ok(Vec::new()) // Devolver un TOC vacío si no se encuentra
     // Err(EpubError::TocNotFound) // O devolver error si prefieres que falle
}


// Parsea un archivo nav.xhtml (EPUB 3)
fn parse_nav_xhtml(content: &str, root_path: &str, nav_file_path: &str) -> Result<Vec<TocEntry>, EpubError> {
    let document = scraper::Html::parse_document(content);
    // Selector robusto: busca un <nav> con epub:type="toc", luego su <ol>, luego <li><a>
    // O directamente busca los enlaces dentro del <nav epub:type="toc">
     let nav_toc_selector = scraper::Selector::parse(r#"nav[epub|type="toc"] ol li a"#)
        .or_else(|_| scraper::Selector::parse(r#"nav[type="toc"] ol li a"#)) // Sin namespace
        .map_err(|e| EpubError::TocParseError(format!("Selector nav inválido: {}", e)))?;

    let mut toc = Vec::new();
    let nav_base_path = Path::new(nav_file_path).parent().unwrap_or_else(|| Path::new(""));

    for element in document.select(&nav_toc_selector) {
        if let Some(href_attr) = element.value().attr("href") {
            let label = element.text().collect::<String>().trim().to_string();
            if label.is_empty() || href_attr.is_empty() {
                continue; // Ignora entradas sin etiqueta o href
            }

            // Resuelve la ruta relativa al archivo nav.xhtml, luego relativa al root_path
            let resolved_href = resolve_relative_path(nav_base_path, href_attr);
             // Normalizamos para comparar con manifest hrefs (que son relativos a root_path)
            let final_href = build_full_path(root_path, &resolved_href.to_string_lossy());


            toc.push(TocEntry {
                label,
                href: final_href, // Guardamos la ruta normalizada relativa al root
                id: element.value().id().map(str::to_string),
            });
        }
    }

    Ok(toc)
}

// Parsea un archivo toc.ncx (EPUB 2)
fn parse_ncx(content: &str, root_path: &str, ncx_file_path: &str) -> Result<Vec<TocEntry>, EpubError> {
    let doc = Document::parse(content)?;
    let nav_map_node = doc.descendants()
        .find(|n| n.tag_name().name() == "navMap")
        .ok_or_else(|| EpubError::TocParseError("No se encontró <navMap> en NCX".to_string()))?;

    let mut toc = Vec::new();
    let ncx_base_path = Path::new(ncx_file_path).parent().unwrap_or_else(|| Path::new(""));

    parse_navpoints(nav_map_node, &mut toc, ncx_base_path, root_path);

    Ok(toc)
}


// Función recursiva para parsear navPoints en NCX
fn parse_navpoints(parent_node: Node, toc: &mut Vec<TocEntry>, ncx_base_path: &Path, root_path: &str) {
    for node in parent_node.children() {
        if node.tag_name().name() == "navPoint" {
             let id = node.attribute("id").map(str::to_string);
            let mut label = "Sin etiqueta".to_string();
             let _href = String::new();

            if let Some(nav_label_node) = node.children().find(|n| n.tag_name().name() == "navLabel") {
                 if let Some(text_node) = nav_label_node.children().find(|n| n.tag_name().name() == "text") {
                     label = text_node.text().unwrap_or("").trim().to_string();
                 }
            }

            if let Some(content_node) = node.children().find(|n| n.tag_name().name() == "content") {
                if let Some(src_attr) = content_node.attribute("src") {
                    if !label.is_empty() && !src_attr.is_empty() {
                         // Resuelve la ruta relativa al archivo ncx, luego relativa al root_path
                         let resolved_href = resolve_relative_path(ncx_base_path, src_attr);
                         let final_href = build_full_path(root_path, &resolved_href.to_string_lossy());

                         toc.push(TocEntry {
                             label,
                             href: final_href,
                             id,
                         });
                    }
                }
            }
             // Recursivamente procesar hijos navPoint anidados (si los hubiera)
             parse_navpoints(node, toc, ncx_base_path, root_path);
        }
    }
}


// --- Funciones auxiliares de rutas ---

// Construye una ruta completa dentro del ZIP relativa al directorio raíz del EPUB.
// root_path: Directorio que contiene el archivo OPF (e.g., "OEBPS").
// relative_href: El href encontrado en OPF o TOC (e.g., "chapter1.xhtml" o "../Text/chapter1.xhtml").
fn build_full_path(root_path: &str, relative_href: &str) -> String {
    if root_path.is_empty() {
        // Si OPF está en la raíz, el href es la ruta final
        normalize_path_simple(relative_href)
    } else {
        // Une el directorio raíz con el href relativo
         let combined = format!("{}/{}", root_path, relative_href);
         normalize_path_simple(&combined)
    }
}


// Resuelve una ruta relativa (`relative_path`) basándose en la ruta del archivo que la contiene (`base_path_str`)
// Devuelve una PathBuf que representa la ruta resuelta.
// NOTA: Esta es una implementación simple. Librerías como `url` o `path_clean` serían más robustas.
fn resolve_relative_path(base_path: &Path, relative_path: &str) -> PathBuf {
     // Si la ruta relativa empieza con '/', es absoluta desde la raíz del "servidor" (zip)
     if relative_path.starts_with('/') {
        return PathBuf::from(relative_path.trim_start_matches('/'));
     }
    let mut current = base_path.to_path_buf();
    // Elimina fragmentos (#...) de la ruta relativa si existen
    let relative_path_no_fragment = relative_path.split('#').next().unwrap_or("");

    for component in relative_path_no_fragment.split('/') {
        match component {
            "." | "" => {} // Ignorar
            ".." => { current.pop(); } // Subir un nivel
            _ => current.push(component), // Añadir componente
        }
    }
     //println!("Resolving relative: base='{}', rel='{}', result='{}'", base_path.display(), relative_path, current.display());
    current
}


// Función helper (simplificada) para normalizar rutas (maneja "./" y "../", asume separador '/')
// Una librería como `path_clean` o `lexiclean` sería más robusta
fn normalize_path_simple(path_str: &str) -> String {
    let mut components: Vec<&str> = Vec::new();
    // Crear una variable vinculada para extender el tiempo de vida
    let normalized = path_str.replace('\\', "/");
    
    for component in normalized.split('/') {
        match component {
            "." | "" => {} // Ignorar componente actual o vacío
            ".." => {
                // No subir más allá de la raíz
                if !components.is_empty() && components.last() != Some(&"..") {
                    components.pop();
                }
            }
            _ => components.push(component), // Añadir componente normal
        }
    }

    // El resto del código permanece igual
    let prefix = if path_str.starts_with('/') { "/" } else { "" };
    
    if components.is_empty() {
        prefix.to_string()
    } else {
        format!("{}{}", prefix, components.join("/"))
    }
}
