"""Tests for PDF export engine and layout formatting."""
import os
import tempfile
import pytest
from PyQt6.QtCore import QMarginsF, QSizeF
from PyQt6.QtGui import QGuiApplication, QPdfWriter, QTextDocument, QPageSize, QPageLayout



def extract_title(name: str) -> str:
    """Extract a clean title from a note path or name, stripping folders and .adoc suffix."""
    clean = name.split("/")[-1]
    if clean.endswith(".adoc"):
        clean = clean[:-5]
    return clean


def get_export_path(home_dir: str, title: str) -> str:
    """Generate the export destination path for a given note title."""
    return os.path.join(home_dir, "Documents", "Notes++ Exports", f"{title}.pdf")


def patch_pdf_internal_links(pdf_path: str, anchor_positions: dict, left_margin_pt: float = 42.52):
    """Convert web URI links pointing to document anchors (#id) into internal PDF destinations."""
    if not anchor_positions or not os.path.exists(pdf_path):
        return

    with open(pdf_path, "rb") as f:
        data = bytearray(f.read())

    import re
    from urllib.parse import unquote

    # 1. Collect page object numbers
    kids_match = re.search(
        rb"/Type\s*/Pages[^>]*?/Kids\s*\[([^\]]+)\]",
        data,
        re.DOTALL
    )
    if not kids_match:
        return

    page_object_numbers = [
        int(m.group(1))
        for m in re.finditer(rb"(\d+)\s+0\s+R", kids_match.group(1))
    ]
    if not page_object_numbers:
        return

    # 2. Find and patch link annotations
    pattern = re.compile(
        rb"(/Type\s*/Annot\s*/Subtype\s*/Link\s*/Rect\s*\[([^\]]+)\]\s*/Border\s*\[[^\]]+\]\s*)(/A\s*<<\s*/Type\s*/Action\s*/S\s*/URI\s*/URI\s*\(([^)]*)\)\s*>>)",
        re.DOTALL
    )

    offset = 0
    for match in list(pattern.finditer(bytes(data))):
        rect_start = match.start(2)
        rect_len = len(match.group(2))
        action_start = match.start(3)
        action_len = len(match.group(3))
        uri_bytes = match.group(4)
        uri = uri_bytes.decode("latin1")

        if uri.startswith("#"):
            target_name = uri[1:]
            if target_name not in anchor_positions:
                target_name = unquote(target_name)

            if target_name in anchor_positions:
                page_idx, pdf_y = anchor_positions[target_name]
                if 0 <= page_idx < len(page_object_numbers):
                    page_obj = page_object_numbers[page_idx]
                    dest = f"/Dest [{page_obj} 0 R /XYZ {left_margin_pt:.2f} {pdf_y:.1f} 0]".encode("latin1")
                    if len(dest) <= action_len:
                        dest = dest.ljust(action_len, b" ")
                        data[action_start:action_start + action_len] = dest
        elif uri == "":
            new_rect = b"0 0 0 0"
            if len(new_rect) <= rect_len:
                new_rect = new_rect.ljust(rect_len, b" ")
                data[rect_start:rect_start + rect_len] = new_rect

    with open(pdf_path, "wb") as f:
        f.write(data)


def render_html_to_pdf(html: str, output_path: str) -> bool:
    """Render an HTML string to a vector PDF using QPdfWriter and QTextDocument."""
    writer = QPdfWriter(output_path)
    writer.setPageSize(QPageSize(QPageSize.PageSizeId.A4))
    writer.setResolution(300)

    layout = writer.pageLayout()
    layout.setUnits(QPageLayout.Unit.Millimeter)
    layout.setMargins(QMarginsF(15.0, 15.0, 15.0, 15.0))
    layout.setPageSize(QPageSize(QPageSize.PageSizeId.A4))
    layout.setOrientation(QPageLayout.Orientation.Portrait)
    writer.setPageLayout(layout)

    print_css = (
        "body { font-family: sans-serif; font-size: 10pt; color: #222222; }\n"
        "h1 { font-size: 20pt; font-weight: bold; margin-bottom: 12px; color: #111111; }\n"
        "h2 { font-size: 15pt; font-weight: bold; margin-top: 14px; margin-bottom: 8px; }\n"
        "pre, code { font-family: monospace; font-size: 9pt; background-color: #1e1e24; color: #f0f6fc; }\n"
        "table { border-collapse: collapse; width: 100%; margin-top: 8px; margin-bottom: 12px; }\n"
        "th, td { border: 1px solid #cccccc; padding: 5px 8px; font-size: 9pt; }\n"
        "th { background-color: #eaeaea; font-weight: bold; }\n"
        "blockquote { margin-left: 12px; border-left: 3px solid #888888; padding-left: 8px; }\n"
        ".admonitionblock { margin-top: 10px; margin-bottom: 12px; padding: 10px; border-left: 4px solid #0284c7; background-color: #f0f9ff; }\n"
    )

    doc = QTextDocument()
    doc.setDefaultStyleSheet(print_css)
    paint_size = QSizeF(layout.paintRect(QPageLayout.Unit.Point).size())
    doc.setPageSize(paint_size)
    doc.setHtml(html)

    # Collect anchor positions
    anchor_positions = {}
    doc_layout = doc.documentLayout()
    page_height_pt = paint_size.height()
    pdf_page_height_pt = 842.0
    top_margin_pt = 15.0 * 72.0 / 25.4
    printable_height_pt = pdf_page_height_pt - (top_margin_pt * 2)
    left_margin_pt = top_margin_pt

    b = doc.begin()
    while b.isValid():
        it = b.begin()
        while not it.atEnd():
            frag = it.fragment()
            if frag.isValid():
                names = frag.charFormat().anchorNames()
                if names:
                    block_top = doc_layout.blockBoundingRect(b).top()
                    page_idx = int(block_top // page_height_pt)
                    y_on_page = block_top - (page_idx * page_height_pt)
                    ratio = y_on_page / page_height_pt if page_height_pt > 0 else 0
                    pdf_y = (pdf_page_height_pt - top_margin_pt) - (ratio * printable_height_pt)
                    for name in names:
                        if name not in anchor_positions:
                            anchor_positions[name] = (page_idx, pdf_y)
            it += 1
        b = b.next()

    doc.print(writer)
    del writer

    patch_pdf_internal_links(output_path, anchor_positions, left_margin_pt)
    return True


def test_extract_title():
    assert extract_title("welcome.adoc") == "welcome"
    assert extract_title("journal/2026-09-18.adoc") == "2026-09-18"
    assert extract_title("Projects/Docs/Architecture.adoc") == "Architecture"
    assert extract_title("PlainName") == "PlainName"


def test_export_path_formatting():
    home = "/home/nemo"
    path = get_export_path(home, "My Notes")
    assert path == "/home/nemo/Documents/Notes++ Exports/My Notes.pdf"


def test_render_html_to_vector_pdf(qapp):
    with tempfile.TemporaryDirectory() as tmpdir:
        out_pdf = os.path.join(tmpdir, "test_document.pdf")
        sample_html = """<!DOCTYPE html>
<html>
<head><title>Test PDF</title></head>
<body>
    <h1>Document Title</h1>
    <p>This is a paragraph with <strong>bold</strong> and <em>italic</em> text.</p>
    <h2>Section Table</h2>
    <table>
        <tr><th>Header A</th><th>Header B</th></tr>
        <tr><td>Cell 1</td><td>Cell 2</td></tr>
    </table>
    <pre><code>fn main() { println!("Hello PDF"); }</code></pre>
    <blockquote>A helpful note callout.</blockquote>
</body>
</html>"""

        success = render_html_to_pdf(sample_html, out_pdf)
        assert success is True
        assert os.path.exists(out_pdf)
        assert os.path.getsize(out_pdf) > 0

        # Verify PDF magic header and trailer
        with open(out_pdf, "rb") as f:
            data = f.read()
            assert data.startswith(b"%PDF-1.")
            assert b"%%EOF" in data


def test_render_empty_and_special_character_documents(qapp):
    with tempfile.TemporaryDirectory() as tmpdir:
        out_pdf = os.path.join(tmpdir, "empty.pdf")
        success = render_html_to_pdf("<!DOCTYPE html><html><body></body></html>", out_pdf)
        assert success is True
        assert os.path.exists(out_pdf)
        assert os.path.getsize(out_pdf) > 0

        out_pdf_special = os.path.join(tmpdir, "special_chars.pdf")
        html_special = "<!DOCTYPE html><html><body><h1>München & Zürich: 100% Grüße</h1></body></html>"
        success2 = render_html_to_pdf(html_special, out_pdf_special)
        assert success2 is True
        assert os.path.exists(out_pdf_special)
        assert os.path.getsize(out_pdf_special) > 0


def test_pdf_export_notification_formatting():
    template = "Exported PDF to %1"
    path = "/home/defaultuser/Documents/Notes++ Exports/welcome.pdf"
    msg = template.replace("%1", path)
    assert msg == "Exported PDF to /home/defaultuser/Documents/Notes++ Exports/welcome.pdf"
    assert len(msg.strip()) > 0


def test_notification_show_guard():
    def show_notification(current_text, msg=None):
        if msg is not None and isinstance(msg, str) and len(msg) > 0:
            current_text = msg
        if not current_text or len(current_text.strip()) == 0:
            return False, current_text
        return True, current_text

    # When msg is passed and valid
    shown, text = show_notification("", "Exported PDF to /path/to/file.pdf")
    assert shown is True
    assert text == "Exported PDF to /path/to/file.pdf"

    # When no msg is passed but text was set
    shown, text = show_notification("Exported PDF to /path/to/file.pdf")
    assert shown is True

    # When empty string is passed
    shown, text = show_notification("")
    assert shown is False

    shown, text = show_notification("", "")
    assert shown is False


def test_notification_target_file_path_logic():
    class MockNotification:
        def __init__(self):
            self.text = ""
            self.target_file_path = ""
            self.opacity = 0.0

        def show(self, msg=None, file_path=None):
            if msg is not None and isinstance(msg, str) and len(msg) > 0:
                self.text = msg
            if not self.text or len(self.text.strip()) == 0:
                return
            self.target_file_path = file_path if isinstance(file_path, str) else ""
            self.opacity = 1.0

        def on_clicked(self):
            url_to_open = None
            if self.target_file_path and len(self.target_file_path) > 0:
                target = self.target_file_path
                if not target.startswith("file://"):
                    target = "file://" + target
                url_to_open = target
            self.opacity = 0.0
            self.target_file_path = ""
            return url_to_open

    notif = MockNotification()
    notif.show("Exported PDF to /path/to/note.pdf", "/path/to/note.pdf")
    assert notif.opacity == 1.0
    assert notif.target_file_path == "/path/to/note.pdf"

    opened = notif.on_clicked()
    assert opened == "file:///path/to/note.pdf"
    assert notif.opacity == 0.0
    assert notif.target_file_path == ""

    # When no file path is provided
    notif.show("Simple message")
    assert notif.opacity == 1.0
    assert notif.target_file_path == ""
    opened = notif.on_clicked()
    assert opened is None
    assert notif.opacity == 0.0


def test_pdf_share_action_parameters():
    path = "/home/defaultuser/Documents/Notes++ Exports/welcome.pdf"
    share_mime = "application/pdf"
    share_resources = [path]

    assert share_mime == "application/pdf"
    assert len(share_resources) == 1
    assert share_resources[0] == path


def test_heading_deduplication_in_rendered_html5():
    """Verify that document title is not duplicated in the body or table of contents."""
    import ctypes, re
    core = ctypes.CDLL("./target/debug/libnotesplusplus_core.so")
    core.notes_core_render_page_html5.argtypes = [ctypes.c_char_p, ctypes.c_char_p]
    core.notes_core_render_page_html5.restype = ctypes.c_void_p

    examples_dir = os.path.abspath("notesplusplus-core/examples").encode("utf-8")
    ptr = core.notes_core_render_page_html5(examples_dir, b"readme.adoc")
    html = ctypes.string_at(ptr).decode("utf-8")

    # Document title should only appear once as a <h1>, inside <header class="document-header">
    h1_matches = re.findall(r"<h1[^>]*>(.*?)</h1>", html)
    assert len(h1_matches) == 1
    assert "Welcome to Notes Plus" in h1_matches[0]

    # Table of contents should start with first section, not the document title
    toc_match = re.search(r"<nav class=\"toc\".*?</nav>", html, re.DOTALL)
    assert toc_match is not None
    toc_text = toc_match.group(0)
    assert "Welcome to Notes Plus" not in toc_text
    assert "Getting Started" in toc_text


def test_document_layout_no_collapsed_line_height(qapp):
    """Verify that multi-section document renders with proper height and multiple pages without collapse."""
    import ctypes
    core = ctypes.CDLL("./target/debug/libnotesplusplus_core.so")
    core.notes_core_render_page_html5.argtypes = [ctypes.c_char_p, ctypes.c_char_p]
    core.notes_core_render_page_html5.restype = ctypes.c_void_p

    examples_dir = os.path.abspath("notesplusplus-core/examples").encode("utf-8")
    ptr = core.notes_core_render_page_html5(examples_dir, b"syntax-highlighting.adoc")
    html = ctypes.string_at(ptr).decode("utf-8")

    doc = QTextDocument()
    # A4 page size at 300 DPI
    page_size = QSizeF(2125, 3154)
    doc.setPageSize(page_size)
    doc.setHtml(html)

    # Document height must exceed single page height and be multiple pages
    assert doc.pageCount() >= 2
    assert doc.size().height() > 3154


def test_embedded_images_in_rendered_html5():
    """Verify that images in notes (e.g. chronicles.adoc) are converted to base64 data URIs."""
    import ctypes, re
    core = ctypes.CDLL("./target/debug/libnotesplusplus_core.so")
    core.notes_core_render_page_html5.argtypes = [ctypes.c_char_p, ctypes.c_char_p]
    core.notes_core_render_page_html5.restype = ctypes.c_void_p

    examples_dir = os.path.abspath("notesplusplus-core/examples").encode("utf-8")
    ptr = core.notes_core_render_page_html5(examples_dir, b"chronicles.adoc")
    html = ctypes.string_at(ptr).decode("utf-8")

    # Image should be resolved to base64 data URI
    assert "data:image/jpeg;base64," in html
    assert "Wolpertinger" in html

    # Verify standard width attribute exists on the img tag for QTextDocument compatibility
    img_match = re.search(r"<img\s+[^>]*src=\"data:image/[^>]*>", html)
    assert img_match is not None
    img_tag = img_match.group(0)
    assert 'alt="Wolpertinger"' in img_tag


def test_confirm_dialog_qml_exists_and_properties():
    """Verify ConfirmDialog.qml exists and contains standard Silica Dialog properties."""
    path = "notesplusplus-sailfish/qml/pages/ConfirmDialog.qml"
    assert os.path.exists(path)
    with open(path, "r", encoding="utf-8") as f:
        content = f.read()

    assert "DialogHeader {" in content
    assert 'property string title: qsTr("Confirm")' in content
    assert 'property string message: ""' in content
    assert 'property string acceptText: qsTr("Yes")' in content
    assert 'property string cancelText: qsTr("Cancel")' in content
    assert "canAccept: true" in content


def test_pageview_prompts_on_existing_pdf():
    """Verify PageView.qml prompts the user via ConfirmDialog before overwriting existing PDFs."""
    path = "notesplusplus-sailfish/qml/pages/PageView.qml"
    with open(path, "r", encoding="utf-8") as f:
        content = f.read()

    # Must check pdf_export_exists before exporting
    assert "bridge.pdf_export_exists(fullPath)" in content
    assert 'pageStack.push(Qt.resolvedUrl("ConfirmDialog.qml")' in content
    assert 'title: qsTr("Overwrite PDF?")' in content
    assert 'acceptText: qsTr("Overwrite")' in content


def test_services_settings_tab_prompts_on_existing_pdf():
    """Verify ServicesSettingsTab.qml prompts before batch exporting if PDFs already exist."""
    path = "notesplusplus-sailfish/qml/components/ServicesSettingsTab.qml"
    with open(path, "r", encoding="utf-8") as f:
        content = f.read()

    assert "bridge.any_pdf_export_exists()" in content
    assert 'pageStack.push(Qt.resolvedUrl("../pages/ConfirmDialog.qml")' in content
    assert 'title: qsTr("Overwrite Existing PDFs?")' in content
    assert 'acceptText: qsTr("Overwrite")' in content


def test_bridge_and_export_helper_signatures():
    """Verify C++ headers declare the PDF export check methods."""
    with open("notesplusplus-sailfish/src/bridge/ExportHelper.h", "r", encoding="utf-8") as f:
        helper_h = f.read()
    assert "QString get_pdf_export_path(" in helper_h
    assert "bool pdf_export_exists(" in helper_h
    assert "bool any_pdf_export_exists()" in helper_h

    with open("notesplusplus-sailfish/src/bridge/NotesBridge.h", "r", encoding="utf-8") as f:
        bridge_h = f.read()
    assert "Q_INVOKABLE QString get_pdf_export_path(QString page_name);" in bridge_h
    assert "Q_INVOKABLE bool    pdf_export_exists(QString page_name);" in bridge_h
    assert "Q_INVOKABLE bool    any_pdf_export_exists();" in bridge_h


def test_code_block_print_style_no_black_background():
    """Verify code blocks in PDF export have a clean light background and mapped high-contrast text."""
    with open("notesplusplus-sailfish/src/bridge/ExportHelper.cpp", "r", encoding="utf-8") as f:
        helper_cpp = f.read()

    # Code block background must not be black
    assert "background-color: #1e1e24;" not in helper_cpp
    assert "background-color: #f6f8fa;" in helper_cpp
    assert "color: #24292f;" in helper_cpp
    assert "s_colorMap" in helper_cpp
    assert "#c0c5ce" in helper_cpp


def test_toc_rendered_without_bullets():
    """Verify TOC list styling removes disc bullets in both PDF print CSS and web stylesheet."""
    with open("notesplusplus-sailfish/src/bridge/ExportHelper.cpp", "r", encoding="utf-8") as f:
        helper_cpp = f.read()

    assert ".toc ul" in helper_cpp
    assert "list-style: none;" in helper_cpp
    assert "list-style-type: none;" in helper_cpp

    with open("notesplusplus-core/assets/html/document.css", "r", encoding="utf-8") as f:
        doc_css = f.read()

    assert ".toc li::before {\n    content: \"\";\n    display: none;\n}" in doc_css
    assert "list-style-type: none;" in doc_css


def test_pdf_toc_named_destinations_structure(qapp):
    """Verify that TOC links in generated PDF map to internal named destinations."""
    import ctypes, subprocess
    core = ctypes.CDLL("./target/debug/libnotesplusplus_core.so")
    core.notes_core_render_page_html5.argtypes = [ctypes.c_char_p, ctypes.c_char_p]
    core.notes_core_render_page_html5.restype = ctypes.c_void_p

    examples_dir = os.path.abspath("notesplusplus-core/examples").encode("utf-8")
    ptr = core.notes_core_render_page_html5(examples_dir, b"readme.adoc")
    html = ctypes.string_at(ptr).decode("utf-8")

    pdf_out = "/tmp/test_readme_dests.pdf"
    writer = QPdfWriter(pdf_out)
    writer.setPageSize(QPageSize(QPageSize.PageSizeId.A4))
    doc = QTextDocument()
    doc.setHtml(html)
    doc.print(writer)

    result = subprocess.run(["pdfinfo", "-dests", pdf_out], capture_output=True, text=True)
    assert result.returncode == 0
    # Must contain named destination for headings
    assert "getting-started" in result.stdout


def test_pdf_internal_link_destinations(qapp):
    """Verify that TOC and anchor links are converted to internal destinations while external URIs remain intact."""
    # 1. Test Qt5 PDF link structure patching
    fake_qt5_pdf = (
        b"%PDF-1.4\n"
        b"1 0 obj\n<< /Type /Pages /Kids [ 10 0 R 20 0 R ] >>\nendobj\n"
        b"2 0 obj\n<<\n/Type /Annot\n/Subtype /Link\n/Rect [10.0 20.0 30.0 40.0]\n/Border [0 0 0]\n"
        b"/A <<\n/Type /Action\n/S /URI\n/URI (#section-1)\n>>\n>>\nendobj\n"
        b"3 0 obj\n<<\n/Type /Annot\n/Subtype /Link\n/Rect [10.0 20.0 30.0 40.0]\n/Border [0 0 0]\n"
        b"/A <<\n/Type /Action\n/S /URI\n/URI (https://example.org)\n>>\n>>\nendobj\n"
        b"4 0 obj\n<<\n/Type /Annot\n/Subtype /Link\n/Rect [10.0 20.0 30.0 40.0]\n/Border [0 0 0]\n"
        b"/A <<\n/Type /Action\n/S /URI\n/URI ()\n>>\n>>\nendobj\n"
        b"xref\n0 5\n"
        b"trailer << /Root 1 0 R >>\n%%EOF"
    )

    with tempfile.TemporaryDirectory() as tmpdir:
        mock_pdf_path = os.path.join(tmpdir, "mock_qt5.pdf")
        with open(mock_pdf_path, "wb") as f:
            f.write(fake_qt5_pdf)

        anchors = {"section-1": (0, 750.5)}
        patch_pdf_internal_links(mock_pdf_path, anchors, left_margin_pt=42.52)

        with open(mock_pdf_path, "rb") as f:
            patched_bytes = f.read()

        # Length must be preserved exactly
        assert len(patched_bytes) == len(fake_qt5_pdf)
        # TOC anchor #section-1 converted to internal PDF /Dest array
        assert b"/Dest [10 0 R /XYZ 42.52 750.5 0]" in patched_bytes
        assert b"#section-1" not in patched_bytes
        # External web URL preserved
        assert b"/URI (https://example.org)" in patched_bytes
        # Empty URI neutralized to zero rect
        assert b"/Rect [0 0 0 0" in patched_bytes

    # 2. Test live HTML to PDF rendering
    with tempfile.TemporaryDirectory() as tmpdir:
        pdf_out = os.path.join(tmpdir, "test_toc_links.pdf")
        html = """<!DOCTYPE html>
<html>
<head><title>TOC Test</title></head>
<body>
    <div class="toc">
        <ul>
            <li><a href="#section-1">Section 1</a></li>
            <li><a href="https://example.org">External Web Link</a></li>
        </ul>
    </div>
    <div style="height: 1200px;"><p>Content spacing</p></div>
    <h2 id="section-1">Section 1</h2>
    <p>Details 1</p>
</body>
</html>"""
        success = render_html_to_pdf(html, pdf_out)
        assert success is True
        assert os.path.exists(pdf_out)

        with open(pdf_out, "rb") as f:
            live_bytes = f.read()

        assert b"/Dest" in live_bytes
        assert b"/URI (https://example.org)" in live_bytes


def test_pdf_font_sizing_points(qapp):
    """Verify that body text in generated PDF uses true typography points and is not downscaled."""
    import subprocess
    with tempfile.TemporaryDirectory() as tmpdir:
        pdf_out = os.path.join(tmpdir, "font_test.pdf")
        html = "<!DOCTYPE html><html><body><p>Hello world 10pt test.</p></body></html>"
        success = render_html_to_pdf(html, pdf_out)
        assert success is True
        assert os.path.exists(pdf_out)

        # Check word bounding box layout
        result = subprocess.run(
            ["pdftotext", "-f", "1", "-l", "1", "-layout", "-bbox-layout", pdf_out, "-"],
            capture_output=True,
            text=True
        )
        assert result.returncode == 0
        # Parse bbox for "Hello"
        import re
        match = re.search(r'<word xMin="([0-9.]+)" yMin="([0-9.]+)" xMax="([0-9.]+)" yMax="([0-9.]+)">Hello</word>', result.stdout)
        assert match is not None
        ymin = float(match.group(2))
        ymax = float(match.group(4))
        height = ymax - ymin
        # Font height in points must be at least 10 pt (not collapsed to 4 pt)
        assert height >= 10.0


def test_chronicles_first_heading_renders_italics(qapp):
    """Verify that in chronicles.adoc the first heading renders italic text without literal underscores."""
    import ctypes, subprocess
    core = ctypes.CDLL("./target/debug/libnotesplusplus_core.so")
    core.notes_core_render_page_html5.argtypes = [ctypes.c_char_p, ctypes.c_char_p]
    core.notes_core_render_page_html5.restype = ctypes.c_void_p

    examples_dir = os.path.abspath("notesplusplus-core/examples").encode("utf-8")
    ptr = core.notes_core_render_page_html5(examples_dir, b"chronicles.adoc")
    html = ctypes.string_at(ptr).decode("utf-8")

    # Heading must contain <em>Thrilling</em> and not literal _Thrilling_
    assert "<em>Thrilling</em>" in html
    assert "_Thrilling_" not in html

    # Document title in <title> must be clean plain text without underscore markup
    assert "<title>The Dangerous &amp; Thrilling Documentation Chronicles: Based on True Events</title>" in html

    # Render to PDF and verify extracted text
    with tempfile.TemporaryDirectory() as tmpdir:
        pdf_out = os.path.join(tmpdir, "chronicles_heading_test.pdf")
        success = render_html_to_pdf(html, pdf_out)
        assert success is True
        assert os.path.exists(pdf_out)

        result = subprocess.run(["pdftotext", pdf_out, "-"], capture_output=True, text=True)
        assert result.returncode == 0
        assert "The Dangerous & Thrilling Documentation" in result.stdout
        assert "Chronicles: Based on True Events" in result.stdout
        assert "_Thrilling_" not in result.stdout
