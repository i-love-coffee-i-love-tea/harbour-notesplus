"""Tests for pure utility functions in composables/utils.js"""
import json
import re
import pytest
from PyQt6.QtCore import QCoreApplication
from PyQt6.QtQml import QJSEngine

_app = None

@pytest.fixture(scope="session")
def engine():
    global _app
    if _app is None:
        _app = QCoreApplication([])
    eng = QJSEngine()
    with open("notesplusplus-core/assets/web/composables/utils.js", "r", encoding="utf-8") as f:
        code = f.read()
    code = re.sub(r"^export\s+", "", code, flags=re.MULTILINE)
    # Strip async function (QJSEngine doesn't support async/await)
    code = re.sub(r"// SSE stream consumer.*?\nasync function consumeSseStream\(.*?\n\}", "", code, flags=re.DOTALL)
    res = eng.evaluate(code)
    if res.isError():
        raise RuntimeError(f"Failed to evaluate utils.js: {res.toString()}")
    return eng


# ── slugify ──────────────────────────────────────────────────

def test_slugify_normal(engine):
    assert engine.evaluate('slugify("Hello World")').toString() == "hello-world"

def test_slugify_special_chars(engine):
    assert engine.evaluate('slugify("Hello, World! @2024")').toString() == "hello-world-2024"

def test_slugify_empty(engine):
    assert engine.evaluate('slugify("")').toString() == "untitled"

def test_slugify_whitespace(engine):
    assert engine.evaluate('slugify("   ")').toString() == "untitled"

def test_slugify_already_slug(engine):
    assert engine.evaluate('slugify("hello-world")').toString() == "hello-world"

def test_slugify_leading_trailing_hyphens(engine):
    assert engine.evaluate('slugify("--foo--")').toString() == "foo"


# ── titleFromPath ────────────────────────────────────────────

def test_titleFromPath_url(engine):
    assert engine.evaluate('titleFromPath("https://example.com/docs/my-page.html")').toString() == "My page"

def test_titleFromPath_url_hostname_only(engine):
    assert engine.evaluate('titleFromPath("https://example.com")').toString() == "example.com"

def test_titleFromPath_file_path(engine):
    assert engine.evaluate('titleFromPath("/home/user/my_file.txt")').toString() == "My file"

def test_titleFromPath_empty(engine):
    assert engine.evaluate('titleFromPath("")').toString() == ""


# ── isExternalUrlStr ─────────────────────────────────────────

def test_isExternalUrlStr_http(engine):
    assert engine.evaluate('isExternalUrlStr("http://example.com")').toBool() is True

def test_isExternalUrlStr_https(engine):
    assert engine.evaluate('isExternalUrlStr("https://example.com")').toBool() is True

def test_isExternalUrlStr_mailto(engine):
    assert engine.evaluate('isExternalUrlStr("mailto:foo@bar.com")').toBool() is True

def test_isExternalUrlStr_ftp(engine):
    assert engine.evaluate('isExternalUrlStr("ftp://files.example.com")').toBool() is True

def test_isExternalUrlStr_relative(engine):
    assert engine.evaluate('isExternalUrlStr("some-file.adoc")').toBool() is False

def test_isExternalUrlStr_hash(engine):
    assert engine.evaluate('isExternalUrlStr("#section")').toBool() is False

def test_isExternalUrlStr_empty(engine):
    assert engine.evaluate('isExternalUrlStr("")').toBool() is False


# ── computeFilenameFromQuery ─────────────────────────────────

def test_computeFilenameFromQuery_empty(engine):
    assert engine.evaluate('computeFilenameFromQuery("")').toString() == ""

def test_computeFilenameFromQuery_adoc_suffix(engine):
    assert engine.evaluate('computeFilenameFromQuery("myfile.adoc")').toString() == "myfile.adoc"

def test_computeFilenameFromQuery_spaces(engine):
    assert engine.evaluate('computeFilenameFromQuery("My Notes")').toString() == "my-notes.adoc"

def test_computeFilenameFromQuery_external(engine):
    assert engine.evaluate('computeFilenameFromQuery("https://example.com")').toString() == "https://example.com"


# ── buildLinkPreview ─────────────────────────────────────────

def test_buildLinkPreview_with_filename(engine):
    js = '''buildLinkPreview({filename: "test.adoc", title: "Test", displayText: "", query: "", isExternal: false, customFilename: ""})'''
    assert engine.evaluate(js).toString() == "xref:test.adoc[Test]"

def test_buildLinkPreview_external(engine):
    js = '''buildLinkPreview({filename: "", title: "", displayText: "Click", query: "https://example.com", isExternal: true, customFilename: ""})'''
    assert engine.evaluate(js).toString() == "https://example.com[Click]"

def test_buildLinkPreview_empty(engine):
    js = '''buildLinkPreview({filename: "", title: "", displayText: "", query: "", isExternal: false, customFilename: ""})'''
    assert engine.evaluate(js).toString() == ""


# ── parseBlocksFromText ──────────────────────────────────────

def test_parseBlocks_empty(engine):
    res = json.loads(engine.evaluate('JSON.stringify(parseBlocksFromText(""))').toString())
    assert res == ['']

def test_parseBlocks_single_paragraph(engine):
    res = json.loads(engine.evaluate('JSON.stringify(parseBlocksFromText("Hello world"))').toString())
    assert res == ['Hello world']

def test_parseBlocks_two_paragraphs(engine):
    res = json.loads(engine.evaluate('JSON.stringify(parseBlocksFromText("Para 1\\n\\nPara 2"))').toString())
    assert len(res) == 2
    assert res[0] == 'Para 1'
    assert res[1] == 'Para 2'

def test_parseBlocks_delimited_block(engine):
    js = 'JSON.stringify(parseBlocksFromText("before\\n\\n----\\ncode here\\n----\\n\\nafter"))'
    res = json.loads(engine.evaluate(js).toString())
    assert len(res) == 3
    assert '----' in res[1]


# ── formatSessionRemaining ───────────────────────────────────

def test_formatSessionRemaining_zero(engine):
    assert engine.evaluate('formatSessionRemaining(0)').toString() == "Expired"

def test_formatSessionRemaining_negative(engine):
    assert engine.evaluate('formatSessionRemaining(-10)').toString() == "Expired"

def test_formatSessionRemaining_seconds(engine):
    assert engine.evaluate('formatSessionRemaining(45)').toString() == "45s"

def test_formatSessionRemaining_minutes(engine):
    assert engine.evaluate('formatSessionRemaining(125)').toString() == "2m 5s"

def test_formatSessionRemaining_hours(engine):
    assert engine.evaluate('formatSessionRemaining(3661)').toString() == "1h 1m"

def test_formatSessionRemaining_days(engine):
    assert engine.evaluate('formatSessionRemaining(90000)').toString() == "1d 1h"


# ── formatSessionRemainingFull ───────────────────────────────

def test_formatSessionRemainingFull_zero(engine):
    assert engine.evaluate('formatSessionRemainingFull(0)').toString() == "Expired"

def test_formatSessionRemainingFull_seconds(engine):
    assert engine.evaluate('formatSessionRemainingFull(5)').toString() == "5 sec"

def test_formatSessionRemainingFull_minutes(engine):
    res = engine.evaluate('formatSessionRemainingFull(125)').toString()
    assert "2 min" in res and "5 sec" in res

def test_formatSessionRemainingFull_hours(engine):
    res = engine.evaluate('formatSessionRemainingFull(3661)').toString()
    assert "1 hr" in res and "1 min" in res

def test_formatSessionRemainingFull_days(engine):
    res = engine.evaluate('formatSessionRemainingFull(90000)').toString()
    assert "1 day" in res


# ── formatMarkdown ───────────────────────────────────────────

def test_formatMarkdown_empty(engine):
    assert engine.evaluate('formatMarkdown("")').toString() == ""
    assert engine.evaluate('formatMarkdown(null)').toString() == ""

def test_formatMarkdown_bold(engine):
    assert engine.evaluate('formatMarkdown("**bold**")').toString() == "<strong>bold</strong>"

def test_formatMarkdown_italic(engine):
    assert engine.evaluate('formatMarkdown("*italic*")').toString() == "<em>italic</em>"

def test_formatMarkdown_code(engine):
    assert engine.evaluate('formatMarkdown("`code`")').toString() == "<code>code</code>"

def test_formatMarkdown_html_escape(engine):
    assert engine.evaluate('formatMarkdown("<script>")').toString() == "&lt;script&gt;"

def test_formatMarkdown_newline(engine):
    assert engine.evaluate('formatMarkdown("a\\nb")').toString() == "a<br>b"


# ── extractSlides ────────────────────────────────────────────

def test_extractSlides_empty(engine):
    res = json.loads(engine.evaluate('JSON.stringify(extractSlides(""))').toString())
    assert len(res) == 1
    assert res[0]["title"] == "Slide 1"

def test_extractSlides_single_heading(engine):
    res = json.loads(engine.evaluate('JSON.stringify(extractSlides("= Title\\n\\nContent"))').toString())
    assert len(res) == 1
    assert res[0]["title"] == "Title"

def test_extractSlides_multiple_headings(engine):
    res = json.loads(engine.evaluate('JSON.stringify(extractSlides("= A\\n\\nfoo\\n\\n== B\\n\\nbar"))').toString())
    assert len(res) == 2
    assert res[0]["title"] == "A"
    assert res[1]["title"] == "B"

def test_extractSlides_page_break(engine):
    res = json.loads(engine.evaluate('JSON.stringify(extractSlides("Part 1\\n\\n<<<\\n\\nPart 2"))').toString())
    assert len(res) == 2


# ── highlightSearchTerms ─────────────────────────────────────

def test_highlightSearchTerms_empty(engine):
    assert engine.evaluate('highlightSearchTerms("", "term")').toString() == ""
    assert engine.evaluate('highlightSearchTerms(null, "term")').toString() == ""
    assert engine.evaluate('highlightSearchTerms("<p>Hello</p>", "")').toString() == "<p>Hello</p>"
    assert engine.evaluate('highlightSearchTerms("<p>Hello</p>", "   ")').toString() == "<p>Hello</p>"

def test_highlightSearchTerms_basic(engine):
    js = 'highlightSearchTerms("<p>Hello world</p>", "world")'
    assert engine.evaluate(js).toString() == '<p>Hello <mark class="search-match">world</mark></p>'

def test_highlightSearchTerms_case_insensitive(engine):
    js = 'highlightSearchTerms("<p>HELLO WORLD</p>", "world")'
    assert engine.evaluate(js).toString() == '<p>HELLO <mark class="search-match">WORLD</mark></p>'

def test_highlightSearchTerms_skips_tags(engine):
    js = 'highlightSearchTerms(\'<a href="https://example.com/world" title="world">world text</a>\', "world")'
    assert engine.evaluate(js).toString() == '<a href="https://example.com/world" title="world"><mark class="search-match">world</mark> text</a>'

def test_highlightSearchTerms_multi_term(engine):
    js = 'highlightSearchTerms("<p>The quick brown fox jumps</p>", "quick fox")'
    res = engine.evaluate(js).toString()
    assert res == '<p>The <mark class="search-match">quick</mark> brown <mark class="search-match">fox</mark> jumps</p>'

def test_highlightSearchTerms_regex_special_chars(engine):
    js = 'highlightSearchTerms("<p>Price is $10.00 (tax included)</p>", "$10.00")'
    assert engine.evaluate(js).toString() == '<p>Price is <mark class="search-match">$10.00</mark> (tax included)</p>'
