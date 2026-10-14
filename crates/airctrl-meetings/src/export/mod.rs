pub mod docx;
pub mod markdown;
pub mod pdf;

use std::path::Path;
use serde::{Deserialize, Serialize};
use crate::error::{MeetingError, Result};
pub use docx::DocxExporter;
pub use markdown::MarkdownExporter;
pub use pdf::PdfExporter;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExportFormat {
    #[serde(rename = "Markdown")]
    Markdown,
    #[serde(rename = "PDF")]
    Pdf,
    #[serde(rename = "DOCX")]
    Docx,
}

impl ExportFormat {
    pub const ALL: [ExportFormat; 3] = [Self::Markdown, Self::Pdf, Self::Docx];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Markdown => "Markdown",
            Self::Pdf => "PDF",
            Self::Docx => "DOCX",
        }
    }
}

impl std::str::FromStr for ExportFormat {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "markdown" | "md" => Ok(Self::Markdown),
            "pdf" => Ok(Self::Pdf),
            "docx" | "doc" => Ok(Self::Docx),
            _ => Err(format!("Unsupported format: {}", s)),
        }
    }
}

pub struct DocumentExporter;

impl DocumentExporter {
    pub fn export(content: &str, format: ExportFormat) -> Result<Vec<u8>> {
        match format {
            ExportFormat::Markdown => Ok(MarkdownExporter::export(content)),
            ExportFormat::Pdf => Ok(PdfExporter::export(content)),
            ExportFormat::Docx => Ok(DocxExporter::export(content)),
        }
    }

    pub fn export_to_file(content: &str, format: ExportFormat, path: &Path) -> Result<()> {
        let bytes = Self::export(content, format)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| MeetingError::ExportIoError(format!("Failed to create parent dir: {e}")))?;
        }
        std::fs::write(path, bytes)
            .map_err(|e| MeetingError::ExportIoError(format!("Failed to write export file: {e}")))?;
        Ok(())
    }
}
