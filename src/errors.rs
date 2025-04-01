// src/errors.rs
use thiserror::Error;
use std::path::PathBuf;

#[derive(Error, Debug)]
pub enum EpubError {
    #[error("Error de I/O: {0}")]
    Io(#[from] std::io::Error),

    #[error("Error al procesar archivo ZIP: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("Error al parsear XML: {0}")]
    Xml(#[from] roxmltree::Error),

    #[error("No se encontró el archivo META-INF/container.xml en el EPUB")]
    MissingContainerXml,

    #[error("No se pudo encontrar el elemento 'rootfile' en container.xml")]
    MissingRootfileElement,

    #[error("No se pudo encontrar el atributo 'full-path' en el elemento 'rootfile'")]
    MissingFullPathAttribute,

    #[error("Archivo OPF no encontrado en el ZIP: {0}")]
    #[allow(dead_code)]
    OpfNotFound(String),

    
    ncontrar el elemento 'package' en el archivo OPF")]
    MissingPackageElement,

    #[error("No se pudo encontrar el elemento 'manifest' en el archivo OPF")]
    MissingManifestElement,

    #[error("No se pudo encontrar el elemento 'spine' en el archivo OPF")]
    MissingSpineElement,

    #[error("No se pudo encontrar el elemento 'metadata' en el archivo OPF")]
    MissingMetadataElement,

    #[error("No se pudo encontrar el item del manifiesto con ID: {0}")]
    ManifestItemNotFound(String),

    #[error("No se pudo encontrar el archivo TOC (ni nav.xhtml ni toc.ncx)")]
    #[allow(dead_code)]
    TocNotFound,

    #[error("No se pudo leer el archivo de contenido: {0}")]
    ContentReadError(String),

    #[error("Índice de capítulo fuera de rango: {0}")]
    InvalidChapterIndex(usize),

    #[error("Ruta inválida encontrada: {0}")]
    #[allow(dead_code)]
    InvalidPath(PathBuf),

    #[error("Error al parsear el índice (TOC): {0}")]
    TocParseError(String),

    #[error("Error al extraer texto de un nodo XML")]
    XmlTextExtractionError,
 }
