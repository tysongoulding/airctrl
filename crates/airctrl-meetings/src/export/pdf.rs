pub struct PdfExporter;

impl PdfExporter {
    pub fn export(content: &str) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(b"%PDF-1.4\n");

        let offset1 = b.len();
        b.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

        let offset2 = b.len();
        b.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

        let offset3 = b.len();
        b.extend_from_slice(b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>\nendobj\n");

        let offset4 = b.len();
        let mut stream_content = String::from("BT\n/F1 12 Tf\n50 720 Td\n");
        for line in content.lines() {
            let sanitized = line
                .replace('\\', "\\\\")
                .replace('(', "\\(")
                .replace(')', "\\)");
            stream_content.push('(');
            stream_content.push_str(&sanitized);
            stream_content.push_str(") Tj\nT*\n");
        }
        stream_content.push_str("ET\n");
        let stream_bytes = stream_content.as_bytes();

        b.extend_from_slice(
            format!("4 0 obj\n<< /Length {} >>\nstream\n", stream_bytes.len()).as_bytes(),
        );
        b.extend_from_slice(stream_bytes);
        b.extend_from_slice(b"\nendstream\nendobj\n");

        let offset5 = b.len();
        b.extend_from_slice(b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n");

        let xref_offset = b.len();
        b.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
        b.extend_from_slice(format!("{:010} 00000 n \n", offset1).as_bytes());
        b.extend_from_slice(format!("{:010} 00000 n \n", offset2).as_bytes());
        b.extend_from_slice(format!("{:010} 00000 n \n", offset3).as_bytes());
        b.extend_from_slice(format!("{:010} 00000 n \n", offset4).as_bytes());
        b.extend_from_slice(format!("{:010} 00000 n \n", offset5).as_bytes());

        b.extend_from_slice(b"trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n");
        b.extend_from_slice(format!("{}\n%%EOF\n", xref_offset).as_bytes());

        b
    }
}
