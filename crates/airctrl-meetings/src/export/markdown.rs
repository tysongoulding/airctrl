pub struct MarkdownExporter;

impl MarkdownExporter {
    pub fn export(content: &str) -> Vec<u8> {
        content.as_bytes().to_vec()
    }
}
