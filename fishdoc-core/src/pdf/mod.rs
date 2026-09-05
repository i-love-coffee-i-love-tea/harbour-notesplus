pub mod theme;
pub mod html_gen;

use std::collections::BTreeMap;
use std::path::Path;

/// Export an AsciiDoc file to PDF via the html5 → printpdf pipeline.
pub fn export_to_pdf(
    adoc_path: &Path,
    output_path: &Path,
) -> Result<(), String> {
    let content = std::fs::read_to_string(adoc_path)
        .map_err(|e| format!("Failed to read {}: {}", adoc_path.display(), e))?;

    let adoc_dir = adoc_path.parent().unwrap_or(Path::new("."));

    let html = html_gen::adoc_to_styled_html(&content, adoc_dir)?;

    let fonts = load_system_fonts();

    let options = printpdf::GeneratePdfOptions {
        font_embedding: Some(true),
        page_width: Some(210.0),
        page_height: Some(297.0),
        margin_top: Some(25.0),
        margin_right: Some(25.0),
        margin_bottom: Some(25.0),
        margin_left: Some(25.0),
        show_page_numbers: Some(true),
        ..Default::default()
    };

    let mut warnings = Vec::new();
    let doc = printpdf::PdfDocument::from_html(
        &html,
        &BTreeMap::new(),
        &fonts,
        &options,
        &mut warnings,
    )
    .map_err(|e| format!("PDF render error: {}", e))?;

    let save_opts = printpdf::PdfSaveOptions::default();
    let pdf_bytes = doc.save(&save_opts, &mut warnings);

    std::fs::write(output_path, &pdf_bytes)
        .map_err(|e| format!("Failed to write PDF: {}", e))?;

    Ok(())
}

/// Load system TTF fonts for embedding in the PDF.
fn load_system_fonts() -> BTreeMap<String, printpdf::Base64OrRaw> {
    let mut fonts = BTreeMap::new();

    let font_files: &[(&str, &str)] = &[
        ("Lato", "/usr/share/fonts/truetype/lato/Lato-Regular.ttf"),
        ("Lato-Bold", "/usr/share/fonts/truetype/lato/Lato-Bold.ttf"),
        ("Lato-Italic", "/usr/share/fonts/truetype/lato/Lato-Italic.ttf"),
        ("Lato-BoldItalic", "/usr/share/fonts/truetype/lato/Lato-BoldItalic.ttf"),
        ("UbuntuMono", "/usr/share/fonts/truetype/ubuntu/UbuntuMono-R.ttf"),
        ("UbuntuMono-Bold", "/usr/share/fonts/truetype/ubuntu/UbuntuMono-B.ttf"),
        ("UbuntuMono-Italic", "/usr/share/fonts/truetype/ubuntu/UbuntuMono-RI.ttf"),
        ("UbuntuMono-BoldItalic", "/usr/share/fonts/truetype/ubuntu/UbuntuMono-BI.ttf"),
    ];

    for (name, path) in font_files {
        if let Ok(bytes) = std::fs::read(path) {
            fonts.insert(name.to_string(), printpdf::Base64OrRaw::Raw(bytes));
        }
    }

    fonts
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn example_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("examples")
            .join(name)
    }

    #[test]
    fn export_simple_adoc_to_pdf() {
        let adoc = example_path("readme.adoc");
        let out = PathBuf::from("/tmp/test_fishdoc_export.pdf");
        let _ = std::fs::remove_file(&out);

        let result = export_to_pdf(&adoc, &out);
        assert!(result.is_ok(), "export failed: {:?}", result.err());
        assert!(out.exists(), "PDF file not created");

        let bytes = std::fs::read(&out).unwrap();
        assert!(bytes.len() > 100, "PDF too small: {} bytes", bytes.len());
        assert_eq!(&bytes[..5], b"%PDF-", "Not a valid PDF");

        let _ = std::fs::remove_file(&out);
    }

    #[test]
    fn export_invoice_adoc_to_pdf() {
        let adoc = example_path("invoice.adoc");
        let out = PathBuf::from("/tmp/test_fishdoc_invoice.pdf");
        let _ = std::fs::remove_file(&out);

        let result = export_to_pdf(&adoc, &out);
        assert!(result.is_ok(), "export failed: {:?}", result.err());

        let bytes = std::fs::read(&out).unwrap();
        assert!(bytes.len() > 100, "PDF too small: {} bytes", bytes.len());
        assert_eq!(&bytes[..5], b"%PDF-", "Not a valid PDF");

        let _ = std::fs::remove_file(&out);
    }
}
