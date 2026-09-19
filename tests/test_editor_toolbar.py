import os
import re
import pytest
from PyQt6.QtCore import QCoreApplication
from PyQt6.QtQml import QJSEngine

_app = None

@pytest.fixture(scope="session")
def qjs_engine():
    global _app
    if _app is None:
        _app = QCoreApplication([])
    engine = QJSEngine()
    with open("notesplus-sailfish/qml/js/BlockHtmlUtils.js", "r", encoding="utf-8") as f:
        code = f.read()
    # Strip .pragma library
    code = re.sub(r"^\.pragma\s+.*$", "", code, flags=re.MULTILINE)
    res = engine.evaluate(code)
    if res.isError():
        raise RuntimeError(f"Failed to evaluate BlockHtmlUtils.js: {res.toString()}")
    return engine

def test_editor_toolbar_file_contracts():
    path = "notesplus-sailfish/qml/components/editor/EditorToolbar.qml"
    assert os.path.exists(path), f"File {path} does not exist"
    with open(path, "r", encoding="utf-8") as f:
        content = f.read()

    # Verify import of BlockHtmlUtils
    assert 'import "../../js/BlockHtmlUtils.js" as BlockHtmlUtils' in content

    # Verify model elements for outdent, indent, move_up, move_down
    assert 'itemId: "outdent"' in content
    assert 'itemId: "indent"' in content
    assert 'itemId: "move_up"' in content
    assert 'itemId: "move_down"' in content

    # Verify action types
    assert 'actionType: "list_level"' in content
    assert 'actionType: "move_lines"' in content

    # Verify method declarations
    assert "function applyListLevelChange(" in content
    assert "function applyMoveLines(" in content
    assert "BlockHtmlUtils.changeListLevel(" in content
    assert "BlockHtmlUtils.moveLines(" in content

def test_toolbar_applyListLevelChange_dispatch(qjs_engine):
    js = """
    var targetTextArea = {
        text: "* Bullet item",
        cursorPosition: 4,
        selectionStart: 4,
        selectionEnd: 4,
        select: function(s, e) { this.selectionStart = s; this.selectionEnd = e; },
        forceActiveFocus: function() {}
    };

    function applyListLevelChange(delta) {
        if (!targetTextArea) return;
        var res = changeListLevel(
            targetTextArea.text,
            targetTextArea.selectionStart,
            targetTextArea.selectionEnd,
            targetTextArea.cursorPosition,
            delta
        );
        targetTextArea.text = res.text;
        targetTextArea.cursorPosition = res.cursorPosition;
        if (res.selectionStart !== res.selectionEnd && typeof targetTextArea.select === "function") {
            targetTextArea.select(res.selectionStart, res.selectionEnd);
        }
    }

    applyListLevelChange(1);
    var indentedText = targetTextArea.text;
    var indentedPos = targetTextArea.cursorPosition;

    applyListLevelChange(-1);
    var outdentedText = targetTextArea.text;
    var outdentedPos = targetTextArea.cursorPosition;

    [indentedText, indentedPos, outdentedText, outdentedPos]
    """
    res = qjs_engine.evaluate(js)
    assert res.property(0).toString() == "** Bullet item"
    assert res.property(1).toInt() == 5
    assert res.property(2).toString() == "* Bullet item"
    assert res.property(3).toInt() == 4

def test_toolbar_applyMoveLines_dispatch(qjs_engine):
    js = """
    var targetTextArea = {
        text: "Line 1\\nLine 2\\nLine 3",
        cursorPosition: 9,
        selectionStart: 9,
        selectionEnd: 9,
        select: function(s, e) { this.selectionStart = s; this.selectionEnd = e; },
        forceActiveFocus: function() {}
    };

    function applyMoveLines(direction) {
        if (!targetTextArea) return;
        var res = moveLines(
            targetTextArea.text,
            targetTextArea.selectionStart,
            targetTextArea.selectionEnd,
            targetTextArea.cursorPosition,
            direction
        );
        targetTextArea.text = res.text;
        targetTextArea.cursorPosition = res.cursorPosition;
        if (res.selectionStart !== res.selectionEnd && typeof targetTextArea.select === "function") {
            targetTextArea.select(res.selectionStart, res.selectionEnd);
        }
    }

    applyMoveLines(-1);
    var movedUpText = targetTextArea.text;
    var movedUpPos = targetTextArea.cursorPosition;

    applyMoveLines(1);
    var movedDownText = targetTextArea.text;
    var movedDownPos = targetTextArea.cursorPosition;

    [movedUpText, movedUpPos, movedDownText, movedDownPos]
    """
    res = qjs_engine.evaluate(js)
    assert res.property(0).toString() == "Line 2\nLine 1\nLine 3"
    assert res.property(1).toInt() == 2
    assert res.property(2).toString() == "Line 1\nLine 2\nLine 3"
    assert res.property(3).toInt() == 9
