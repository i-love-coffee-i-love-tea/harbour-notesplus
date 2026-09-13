import pytest
import re
from PyQt6.QtCore import QCoreApplication
from PyQt6.QtQml import QJSEngine

_app = None

@pytest.fixture(scope="session")
def qjs_engine():
    global _app
    if _app is None:
        _app = QCoreApplication([])
    engine = QJSEngine()
    with open("notesplusplus-sailfish/qml/js/BlockHtmlUtils.js", "r", encoding="utf-8") as f:
        code = f.read()
    # Strip .pragma library
    code = re.sub(r"^\.pragma\s+.*$", "", code, flags=re.MULTILINE)
    res = engine.evaluate(code)
    if res.isError():
        raise RuntimeError(f"Failed to evaluate BlockHtmlUtils.js: {res.toString()}")
    return engine

def test_findAllMatches_emptyQuery(qjs_engine):
    js = """
    var blocks = [
        { html: "<p>Hello world</p>" },
        { html: "<p>Another world block</p>" }
    ];
    var res1 = findAllMatches(blocks, "");
    var res2 = findAllMatches(blocks, "   ");
    var res3 = findAllMatches([], "world");
    [res1.length, res2.length, res3.length]
    """
    res = qjs_engine.evaluate(js)
    assert res.property(0).toInt() == 0
    assert res.property(1).toInt() == 0
    assert res.property(2).toInt() == 0

def test_findAllMatches_multipleBlocks(qjs_engine):
    js = """
    var blocks = [
        { html: "<p>First target block</p>" },
        { html: "<p>Second block without match</p>" },
        { html: "<p>Third target block</p>" }
    ];
    findAllMatches(blocks, "target")
    """
    res = qjs_engine.evaluate(js)
    assert res.property("length").toInt() == 2
    
    match0 = res.property(0)
    assert match0.property("blockIndex").toInt() == 0
    assert match0.property("matchInBlock").toInt() == 0
    assert match0.property("globalIndex").toInt() == 0

    match1 = res.property(1)
    assert match1.property("blockIndex").toInt() == 2
    assert match1.property("matchInBlock").toInt() == 0
    assert match1.property("globalIndex").toInt() == 1

def test_findAllMatches_multipleOccurrencesInSingleBlock(qjs_engine):
    js = """
    var blocks = [
        { html: "<p>foo and foo and foo</p>" },
        { html: "<p>just foo</p>" }
    ];
    findAllMatches(blocks, "foo")
    """
    res = qjs_engine.evaluate(js)
    assert res.property("length").toInt() == 4

    # Block 0, 3 matches
    assert res.property(0).property("blockIndex").toInt() == 0
    assert res.property(0).property("matchInBlock").toInt() == 0
    assert res.property(0).property("globalIndex").toInt() == 0

    assert res.property(1).property("blockIndex").toInt() == 0
    assert res.property(1).property("matchInBlock").toInt() == 1
    assert res.property(1).property("globalIndex").toInt() == 1

    assert res.property(2).property("blockIndex").toInt() == 0
    assert res.property(2).property("matchInBlock").toInt() == 2
    assert res.property(2).property("globalIndex").toInt() == 2

    # Block 1, 1 match
    assert res.property(3).property("blockIndex").toInt() == 1
    assert res.property(3).property("matchInBlock").toInt() == 0
    assert res.property(3).property("globalIndex").toInt() == 3

def test_countMatchesInHtml_tagBoundarySafety(qjs_engine):
    js = """
    var html = '<a href="test">test</a> and <span class="test">test</span>';
    countMatchesInHtml(html, "test")
    """
    res = qjs_engine.evaluate(js)
    # Only 2 matches in visible text, not 4 (attribute occurrences ignored)
    assert res.toInt() == 2

def test_countMatchesInPlainText(qjs_engine):
    js = """
    var text = "func test() { test(); } // test";
    countMatchesInPlainText(text, "test")
    """
    res = qjs_engine.evaluate(js)
    assert res.toInt() == 3

def test_highlightSearchTerms_twoToneStyles(qjs_engine):
    js = """
    var html = "<p>apple and apple and apple</p>";
    var highlighted = highlightSearchTerms(html, "apple", 1);
    highlighted
    """
    res = qjs_engine.evaluate(js).toString()
    
    orange_style = "background:#ff9800;color:#000;font-weight:bold;padding:0 2px;border-radius:2px;border:1px solid #e65100;"
    yellow_style = "background:#ffeb3b;color:#000;padding:0 1px;border-radius:2px;"

    # 1 orange match, 2 yellow matches
    assert res.count(orange_style) == 1
    assert res.count(yellow_style) == 2

    # Verify occurrence 1 is orange and others are yellow
    parts = res.split("<mark ")
    # parts[0] is "<p>"
    assert yellow_style in parts[1] # match 0
    assert orange_style in parts[2] # match 1 (active)
    assert yellow_style in parts[3] # match 2

def test_highlightPlainText_escapingAndHighlight(qjs_engine):
    js = """
    var raw = "<div> & 'hello' & 'hello'</div>";
    highlightPlainText(raw, "hello", 0)
    """
    res = qjs_engine.evaluate(js).toString()
    
    orange_style = "background:#ff9800;color:#000;font-weight:bold;padding:0 2px;border-radius:2px;border:1px solid #e65100;"
    yellow_style = "background:#ffeb3b;color:#000;padding:0 1px;border-radius:2px;"

    # Raw HTML characters must be escaped
    assert "&lt;div&gt;" in res
    assert "&amp;" in res
    assert "<div>" not in res

    # Occurrence 0 is orange, Occurrence 1 is yellow
    assert res.count(orange_style) == 1
    assert res.count(yellow_style) == 1
    parts = res.split("<mark ")
    assert orange_style in parts[1]
    assert yellow_style in parts[2]

def test_regexSpecialChars_literalMatching(qjs_engine):
    js = """
    var text = "int a[10] = {0}; // a[10] array (c++) $100 * 2?";
    var c1 = countMatchesInPlainText(text, "a[10]");
    var c2 = countMatchesInPlainText(text, "(c++)");
    var c3 = countMatchesInPlainText(text, "$100 * 2?");
    var h = highlightPlainText(text, "a[10]", 0);
    [c1, c2, c3, h.indexOf("<mark") !== -1]
    """
    res = qjs_engine.evaluate(js)
    assert res.property(0).toInt() == 2
    assert res.property(1).toInt() == 1
    assert res.property(2).toInt() == 1
    assert res.property(3).toBool() is True
