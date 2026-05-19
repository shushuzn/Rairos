use crate::handlers::helpers::data_dir;
use crate::protocol::{ToolHandler, ToolInputSchema, ToolProperty};
use async_trait::async_trait;
use serde_json::Value;

fn escape_xml(s: &str) -> String {
    s.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;").replace("\"", "&quot;")
}

fn md_to_docx_xml(markdown: &str) -> String {
    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#
    );
    xml.push_str(r#"<w:document xmlns:wpc="http://schemas.microsoft.com/office/word/2010/wordprocessingCanvas"#);
    xml.push_str(r#" xmlns:cx="http://schemas.microsoft.com/office/drawing/2014/chartex"#);
    xml.push_str(r#" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006""#);
    xml.push_str(r#" xmlns:aink="http://schemas.microsoft.com/office/drawing/2016/ink""#);
    xml.push_str(r#" xmlns:am3d="http://schemas.microsoft.com/office/drawing/2017/model3d""#);
    xml.push_str(r#" xmlns:o="urn:schemas-microsoft-com:office:office""#);
    xml.push_str(r#" xmlns:oel="http://schemas.microsoft.com/office/2019/extlst""#);
    xml.push_str(r#" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationship""#);
    xml.push_str(r#" xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math""#);
    xml.push_str(r#" xmlns:v="urn:schemas-microsoft-com:vml""#);
    xml.push_str(r#" xmlns:wp14="http://schemas.microsoft.com/office/word/2010/wordprocessingDrawing""#);
    xml.push_str(r#" xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing""#);
    xml.push_str(r#" xmlns:w10="urn:schemas-microsoft-com:office:word""#);
    xml.push_str(r#" xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main""#);
    xml.push_str(r#" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml""#);
    xml.push_str(r#" xmlns:w15="http://schemas.microsoft.com/office/word/2012/wordml""#);
    xml.push_str(r#" xmlns:w16cex="http://schemas.microsoft.com/office/word/2018/wordml/cex""#);
    xml.push_str(r#" xmlns:w16cid="http://schemas.microsoft.com/office/word/2016/wordml/cid""#);
    xml.push_str(r#" xmlns:w16="http://schemas.microsoft.com/office/word/2018/wordml""#);
    xml.push_str(r#" xmlns:w16sdtdh="http://schemas.microsoft.com/office/word/2020/wordml/sdtdatahash""#);
    xml.push_str(r#" xmlns:w16se="http://schemas.microsoft.com/office/word/2015/wordml/symex""#);
    xml.push_str(r#" xmlns:wpg="http://schemas.microsoft.com/office/word/2010/wordprocessingGroup""#);
    xml.push_str(r#" xmlns:wpi="http://schemas.microsoft.com/office/word/2010/wordprocessingInk""#);
    xml.push_str(r#" xmlns:wne="http://schemas.microsoft.com/office/word/2006/wordml""#);
    xml.push_str(r#" xmlns:wps="http://schemas.microsoft.com/office/word/2010/wordprocessingShape""#);
    xml.push('>');
    xml.push_str("<w:body>");

    let lines: Vec<&str> = markdown.lines().collect();
    let mut in_list = false;
    let mut list_type = "";
    let _list_count = 0usize;

    for line in &lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if in_list {
                xml.push_str("</w:p>");
                in_list = false;
            }
            continue;
        }

        if let Some(text) = trimmed.strip_prefix("# ") {
            xml.push_str(&format!(
                r#"<w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>{}</w:t></w:r></w:p>"#,
                escape_xml(text)
            ));
        }
        else if let Some(text) = trimmed.strip_prefix("## ") {
            xml.push_str(&format!(
                r#"<w:p><w:pPr><w:pStyle w:val="Heading2"/></w:pPr><w:r><w:t>{}</w:t></w:r></w:p>"#,
                escape_xml(text)
            ));
        }
        else if let Some(text) = trimmed.strip_prefix("### ") {
            xml.push_str(&format!(
                r#"<w:p><w:pPr><w:pStyle w:val="Heading3"/></w:pPr><w:r><w:t>{}</w:t></w:r></w:p>"#,
                escape_xml(text)
            ));
        }
        else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            let text = &trimmed[2..];
            if !in_list || list_type != "bullet" {
                if in_list { xml.push_str("</w:p>"); }
                xml.push_str(r#"<w:p><w:pPr><w:numPr><w:ilvl w:val="0"/><w:numId w:val="1"/></w:numPr></w:pPr>"#);
                in_list = true;
                list_type = "bullet";
            }
            xml.push_str(&format!(
                r#"<w:r><w:t>{}</w:t></w:r></w:p><w:p><w:pPr><w:numPr><w:ilvl w:val="0"/><w:numId w:val="1"/></w:numPr></w:pPr>"#,
                escape_xml(text)
            ));
        }
        else if trimmed.starts_with("1. ") || trimmed.starts_with("1) ") {
            let text = if trimmed.starts_with("1. ") { &trimmed[3..] } else { &trimmed[2..] };
            if !in_list || list_type != "number" {
                if in_list { xml.push_str("</w:p>"); }
                xml.push_str(r#"<w:p><w:pPr><w:numPr><w:ilvl w:val="0"/><w:numId w:val="2"/></w:numPr></w:pPr>"#);
                in_list = true;
                list_type = "number";
            }
            xml.push_str(&format!(
                r#"<w:r><w:t>{}</w:t></w:r></w:p><w:p><w:pPr><w:numPr><w:ilvl w:val="0"/><w:numId w:val="2"/></w:numPr></w:pPr>"#,
                escape_xml(text)
            ));
        }
        else if trimmed == "---" || trimmed == "***" || trimmed == "___" {
            xml.push_str(r#"<w:p><w:pPr><w:pBdr><w:bottom w:val="single" w:sz="6" w:space="1" w:color="CCCCCC"/></w:pBdr></w:pPr></w:p>"#);
        }
        else if let Some(text) = trimmed.strip_prefix("> ") {
            xml.push_str(&format!(
                r#"<w:p><w:pPr><w:ind w:left="720" w:right="720"/><w:jc w:val="both"/></w:pPr><w:r><w:rPr><w:i/><w:color w:val="666666"/></w:rPr><w:t>{}</w:t></w:r></w:p>"#,
                escape_xml(text)
            ));
        }
        else {
            if in_list {
                xml.push_str("</w:p>");
                in_list = false;
            }
            let processed = process_inline_formatting(trimmed);
            xml.push_str(&format!(
                r#"<w:p><w:r>{}</w:r></w:p>"#,
                processed
            ));
        }
    }

    if in_list {
        xml.push_str("</w:p>");
    }

    xml.push_str(r#"<w:sectPr><w:pgSz w:w="12240" w:h="15840"/><w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440" w:header="720" w:footer="720" w:gutter="0"/></w:sectPr>"#);
    xml.push_str("</w:body></w:document>");

    xml
}

fn process_inline_formatting(text: &str) -> String {
    let mut result = String::new();
    let mut chars = text.chars().peekable();
    let mut in_bold = false;
    let mut in_italic = false;

    while let Some(c) = chars.next() {
        if c == '*' {
            let next = chars.peek().copied();
            if next == Some('*') {
                chars.next();
                if in_bold {
                    result.push_str("</w:rPr></w:r><w:r>");
                    in_bold = false;
                } else {
                    result.push_str("<w:r><w:rPr><w:b/></w:rPr>");
                    in_bold = true;
                }
            } else {
                if in_italic {
                    result.push_str("</w:rPr></w:r><w:r>");
                    in_italic = false;
                } else {
                    result.push_str("<w:r><w:rPr><w:i/></w:rPr>");
                    in_italic = true;
                }
            }
        } else if c == '`' {
            result.push_str(&format!("<w:t>{}</w:t>", c));
        } else {
            result.push_str(&escape_xml(&c.to_string()));
        }
    }

    if result.is_empty() {
        result.push_str("<w:t></w:t>");
    }

    if !result.contains("<w:t>") {
        result = format!("<w:t>{}</w:t>", result);
    }

    result
}

fn build_docx(markdown: &str, _title: &str) -> Result<Vec<u8>, String> {
    use std::io::Write;

    let mut buffer = Vec::new();
    {
        let mut zip_writer = zip::ZipWriter::new(std::io::Cursor::new(&mut buffer));

        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .unix_permissions(0o644);

        zip_writer.start_file("[Content_Types].xml", options).map_err(|e| e.to_string())?;
        zip_writer.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#).map_err(|e| e.to_string())?;
        zip_writer.write_all(br#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">"#).map_err(|e| e.to_string())?;
        zip_writer.write_all(br#"<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>"#).map_err(|e| e.to_string())?;
        zip_writer.write_all(br#"<Default Extension="xml" ContentType="application/xml"/>"#).map_err(|e| e.to_string())?;
        zip_writer.write_all(br#"<Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>"#).map_err(|e| e.to_string())?;
        zip_writer.write_all(br#"</Types>"#).map_err(|e| e.to_string())?;

        zip_writer.start_file("_rels/.rels", options).map_err(|e| e.to_string())?;
        zip_writer.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#).map_err(|e| e.to_string())?;
        zip_writer.write_all(br#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#).map_err(|e| e.to_string())?;
        zip_writer.write_all(br#"<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>"#).map_err(|e| e.to_string())?;
        zip_writer.write_all(br#"</Relationships>"#).map_err(|e| e.to_string())?;

        zip_writer.start_file("word/_rels/document.xml.rels", options).map_err(|e| e.to_string())?;
        zip_writer.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#).map_err(|e| e.to_string())?;
        zip_writer.write_all(br#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#).map_err(|e| e.to_string())?;
        zip_writer.write_all(br#"</Relationships>"#).map_err(|e| e.to_string())?;

        zip_writer.start_file("word/document.xml", options).map_err(|e| e.to_string())?;
        let doc_xml = md_to_docx_xml(markdown);
        zip_writer.write_all(doc_xml.as_bytes()).map_err(|e| e.to_string())?;

        zip_writer.start_file("word/settings.xml", options).map_err(|e| e.to_string())?;
        zip_writer.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#).map_err(|e| e.to_string())?;
        zip_writer.write_all(br#"<w:settings xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:defaultTabStop w:val="720"/></w:settings>"#).map_err(|e| e.to_string())?;

        zip_writer.start_file("word/styles.xml", options).map_err(|e| e.to_string())?;
        let styles_xml = build_styles_xml();
        zip_writer.write_all(styles_xml.as_bytes()).map_err(|e| e.to_string())?;

        zip_writer.start_file("word/numbering.xml", options).map_err(|e| e.to_string())?;
        let numbering_xml = build_numbering_xml();
        zip_writer.write_all(numbering_xml.as_bytes()).map_err(|e| e.to_string())?;

        zip_writer.finish().map_err(|e| e.to_string())?;
    }

    Ok(buffer)
}

fn build_styles_xml() -> String {
    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#
    );
    xml.push_str(r#"<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">"#);

    xml.push_str(r#"<w:style w:type="paragraph" w:default="1" w:styleId="Normal">"#);
    xml.push_str(r#"<w:name w:val="Normal"/>"#);
    xml.push_str(r#"<w:qFormat/>"#);
    xml.push_str(r#"</w:style>"#);

    xml.push_str(r#"<w:style w:type="paragraph" w:styleId="Heading1">"#);
    xml.push_str(r#"<w:name w:val="heading 1"/>"#);
    xml.push_str(r#"<w:basedOn w:val="Normal"/>"#);
    xml.push_str(r#"<w:next w:val="Normal"/>"#);
    xml.push_str(r#"<w:qFormat/>"#);
    xml.push_str(r#"<w:pPr><w:outlineLvl w:val="0"/></w:pPr>"#);
    xml.push_str(r#"<w:rPr><w:b/><w:sz w:val="48"/><w:szCs w:val="48"/></w:rPr>"#);
    xml.push_str(r#"</w:style>"#);

    xml.push_str(r#"<w:style w:type="paragraph" w:styleId="Heading2">"#);
    xml.push_str(r#"<w:name w:val="heading 2"/>"#);
    xml.push_str(r#"<w:basedOn w:val="Normal"/>"#);
    xml.push_str(r#"<w:next w:val="Normal"/>"#);
    xml.push_str(r#"<w:qFormat/>"#);
    xml.push_str(r#"<w:pPr><w:outlineLvl w:val="1"/></w:pPr>"#);
    xml.push_str(r#"<w:rPr><w:b/><w:sz w:val="32"/><w:szCs w:val="32"/></w:rPr>"#);
    xml.push_str(r#"</w:style>"#);

    xml.push_str(r#"<w:style w:type="paragraph" w:styleId="Heading3">"#);
    xml.push_str(r#"<w:name w:val="heading 3"/>"#);
    xml.push_str(r#"<w:basedOn w:val="Normal"/>"#);
    xml.push_str(r#"<w:next w:val="Normal"/>"#);
    xml.push_str(r#"<w:qFormat/>"#);
    xml.push_str(r#"<w:pPr><w:outlineLvl w:val="2"/></w:pPr>"#);
    xml.push_str(r#"<w:rPr><w:b/><w:sz w:val="28"/><w:szCs w:val="28"/></w:rPr>"#);
    xml.push_str(r#"</w:style>"#);

    xml.push_str("</w:styles>");
    xml
}

fn build_numbering_xml() -> String {
    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#
    );
    xml.push_str(r#"<w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">"#);

    xml.push_str(r#"<w:abstractNum w:abstractNumId="0">"#);
    xml.push_str(r#"<w:multiLevelType w:val="hybridMultilevel"/>"#);
    xml.push_str(r#"<w:lvl w:ilvl="0"><w:start w:val="1"/><w:numFmt w:val="bullet"/><w:lvlText w:val="&#x2022;"/><w:lvlJc w:val="left"/><w:pPr><w:ind w:left="720" w:hanging="360"/></w:pPr><w:rPr><w:rFonts w:ascii="Arial" w:hAnsi="Arial" w:hint="default"/></w:rPr></w:lvl>"#);
    xml.push_str(r#"</w:abstractNum>"#);

    xml.push_str(r#"<w:abstractNum w:abstractNumId="1">"#);
    xml.push_str(r#"<w:multiLevelType w:val="hybridMultilevel"/>"#);
    xml.push_str(r#"<w:lvl w:ilvl="0"><w:start w:val="1"/><w:numFmt w:val="decimal"/><w:lvlText w:val="%1."/><w:lvlJc w:val="left"/><w:pPr><w:ind w:left="720" w:hanging="360"/></w:pPr></w:lvl>"#);
    xml.push_str(r#"</w:abstractNum>"#);

    xml.push_str(r#"<w:num w:numId="1"><w:abstractNumId w:val="0"/></w:num>"#);
    xml.push_str(r#"<w:num w:numId="2"><w:abstractNumId w:val="1"/></w:num>"#);

    xml.push_str("</w:numbering>");
    xml
}

pub struct PaperDocxHandler;

#[async_trait]
impl ToolHandler for PaperDocxHandler {
    fn name(&self) -> &str { "paper_docx_export" }
    fn description(&self) -> &str { "Export a research paper or literature review from markdown to a Word (.docx) document with proper formatting, headings, lists, and styles" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("markdown".into(), ToolProperty::string("Full markdown content of the paper or review to export")),
                ("title".into(), ToolProperty::string("Document title for the Word file")),
                ("filename".into(), ToolProperty::string("Output filename without extension (default: paper_export)")),
            ].into_iter().collect(),
            vec!["markdown".into(), "title".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let markdown = params["markdown"].as_str().ok_or("Missing markdown")?;
        let title = params["title"].as_str().ok_or("Missing title")?;
        let filename = params.get("filename").and_then(|v| v.as_str()).unwrap_or("paper_export");

        let docx_data = build_docx(markdown, title)?;

        let output_dir = data_dir().join("exports");
        std::fs::create_dir_all(&output_dir).map_err(|e| e.to_string())?;

        let output_path = output_dir.join(format!("{}.docx", filename));
        std::fs::write(&output_path, &docx_data).map_err(|e| e.to_string())?;

        let file_size = docx_data.len();

        Ok(serde_json::json!({
            "title": title,
            "filename": format!("{}.docx", filename),
            "path": output_path.to_string_lossy(),
            "size_bytes": file_size,
            "format": "docx",
        }))
    }
}
