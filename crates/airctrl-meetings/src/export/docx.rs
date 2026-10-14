pub struct DocxExporter;

fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFFFFFF;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB88320 & mask);
        }
    }
    !crc
}

struct ZipEntry {
    name: String,
    data: Vec<u8>,
}

impl DocxExporter {
    pub fn export(content: &str) -> Vec<u8> {
        let content_types = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>"#;

        let rels = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>"#;

        let mut body_xml = String::from(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>"#,
        );

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                body_xml.push_str("<w:p/>");
                continue;
            }

            let escaped = trimmed
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
                .replace('"', "&quot;");

            if let Some(h1) = escaped.strip_prefix("# ") {
                body_xml.push_str(&format!(
                    r#"<w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>{}</w:t></w:r></w:p>"#,
                    h1.trim()
                ));
            } else if let Some(h2) = escaped.strip_prefix("## ") {
                body_xml.push_str(&format!(
                    r#"<w:p><w:pPr><w:pStyle w:val="Heading2"/></w:pPr><w:r><w:t>{}</w:t></w:r></w:p>"#,
                    h2.trim()
                ));
            } else if let Some(item) = escaped.strip_prefix("- [ ] ") {
                body_xml.push_str(&format!(
                    r#"<w:p><w:r><w:t>&#x2610; {}</w:t></w:r></w:p>"#,
                    item.trim()
                ));
            } else if let Some(bullet) = escaped.strip_prefix("- ") {
                body_xml.push_str(&format!(
                    r#"<w:p><w:r><w:t>&#x2022; {}</w:t></w:r></w:p>"#,
                    bullet.trim()
                ));
            } else {
                body_xml.push_str(&format!(
                    r#"<w:p><w:r><w:t>{}</w:t></w:r></w:p>"#,
                    escaped
                ));
            }
        }

        body_xml.push_str("  </w:body>\n</w:document>");

        let entries = vec![
            ZipEntry {
                name: "[Content_Types].xml".to_string(),
                data: content_types.to_vec(),
            },
            ZipEntry {
                name: "_rels/.rels".to_string(),
                data: rels.to_vec(),
            },
            ZipEntry {
                name: "word/document.xml".to_string(),
                data: body_xml.into_bytes(),
            },
        ];

        Self::build_zip(entries)
    }

    fn build_zip(entries: Vec<ZipEntry>) -> Vec<u8> {
        let mut out = Vec::new();
        let mut cd_entries = Vec::new();

        for entry in &entries {
            let offset = out.len() as u32;
            let name_bytes = entry.name.as_bytes();
            let crc = crc32(&entry.data);
            let size = entry.data.len() as u32;

            // Local file header: signature 0x04034b50
            out.extend_from_slice(&[0x50, 0x4b, 0x03, 0x04]);
            out.extend_from_slice(&20u16.to_le_bytes()); // version needed
            out.extend_from_slice(&0u16.to_le_bytes());  // flags
            out.extend_from_slice(&0u16.to_le_bytes());  // compression = Store (0)
            out.extend_from_slice(&0u16.to_le_bytes());  // mod time
            out.extend_from_slice(&0u16.to_le_bytes());  // mod date
            out.extend_from_slice(&crc.to_le_bytes());
            out.extend_from_slice(&size.to_le_bytes()); // compressed size
            out.extend_from_slice(&size.to_le_bytes()); // uncompressed size
            out.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes()); // extra field len
            out.extend_from_slice(name_bytes);
            out.extend_from_slice(&entry.data);

            cd_entries.push((entry, offset, crc, size));
        }

        let cd_offset = out.len() as u32;

        for (entry, offset, crc, size) in &cd_entries {
            let name_bytes = entry.name.as_bytes();
            // Central directory header: signature 0x02014b50
            out.extend_from_slice(&[0x50, 0x4b, 0x01, 0x02]);
            out.extend_from_slice(&20u16.to_le_bytes()); // version made by
            out.extend_from_slice(&20u16.to_le_bytes()); // version needed
            out.extend_from_slice(&0u16.to_le_bytes());  // flags
            out.extend_from_slice(&0u16.to_le_bytes());  // compression = Store (0)
            out.extend_from_slice(&0u16.to_le_bytes());  // mod time
            out.extend_from_slice(&0u16.to_le_bytes());  // mod date
            out.extend_from_slice(&crc.to_le_bytes());
            out.extend_from_slice(&size.to_le_bytes());
            out.extend_from_slice(&size.to_le_bytes());
            out.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes()); // extra field len
            out.extend_from_slice(&0u16.to_le_bytes()); // comment len
            out.extend_from_slice(&0u16.to_le_bytes()); // disk number start
            out.extend_from_slice(&0u16.to_le_bytes()); // internal file attrs
            out.extend_from_slice(&0u32.to_le_bytes()); // external file attrs
            out.extend_from_slice(&offset.to_le_bytes()); // local header offset
            out.extend_from_slice(name_bytes);
        }

        let cd_size = (out.len() as u32) - cd_offset;
        let count = entries.len() as u16;

        // End of central directory record: signature 0x06054b50
        out.extend_from_slice(&[0x50, 0x4b, 0x05, 0x06]);
        out.extend_from_slice(&0u16.to_le_bytes()); // disk number
        out.extend_from_slice(&0u16.to_le_bytes()); // disk with CD start
        out.extend_from_slice(&count.to_le_bytes()); // entries on this disk
        out.extend_from_slice(&count.to_le_bytes()); // total entries
        out.extend_from_slice(&cd_size.to_le_bytes()); // size of CD
        out.extend_from_slice(&cd_offset.to_le_bytes()); // offset of CD
        out.extend_from_slice(&0u16.to_le_bytes()); // comment length

        out
    }
}
