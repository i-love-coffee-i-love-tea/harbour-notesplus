import pytest
import re
import json
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

def eval_js(qjs_engine, expr):
    res = qjs_engine.evaluate(expr)
    if res.isError():
        raise RuntimeError(f"JS Error in '{expr}': {res.toString()}")
    return res

# ---------------------------------------------------------------------------
# 1. indentListLine
# ---------------------------------------------------------------------------

def test_indentListLine_asterisk(qjs_engine):
    assert eval_js(qjs_engine, 'indentListLine("* Item")').toString() == "** Item"
    assert eval_js(qjs_engine, 'indentListLine("** Item")').toString() == "*** Item"
    assert eval_js(qjs_engine, 'indentListLine("*** Item")').toString() == "**** Item"
    assert eval_js(qjs_engine, 'indentListLine("**** Item")').toString() == "***** Item"
    assert eval_js(qjs_engine, 'indentListLine("***** Item")').toString() == "***** Item"

def test_indentListLine_task(qjs_engine):
    assert eval_js(qjs_engine, 'indentListLine("* [ ] Task")').toString() == "** [ ] Task"
    assert eval_js(qjs_engine, 'indentListLine("* [x] Done")').toString() == "** [x] Done"
    assert eval_js(qjs_engine, 'indentListLine("** [ ] Subtask")').toString() == "*** [ ] Subtask"
    assert eval_js(qjs_engine, 'indentListLine("***** [ ] Max")').toString() == "***** [ ] Max"

def test_indentListLine_dot(qjs_engine):
    assert eval_js(qjs_engine, 'indentListLine(". First")').toString() == ".. First"
    assert eval_js(qjs_engine, 'indentListLine(".. Second")').toString() == "... Second"
    assert eval_js(qjs_engine, 'indentListLine("... Third")').toString() == ".... Third"
    assert eval_js(qjs_engine, 'indentListLine(".... Fourth")').toString() == "..... Fourth"
    assert eval_js(qjs_engine, 'indentListLine("..... Fifth")').toString() == "..... Fifth"

def test_indentListLine_hyphen_and_number(qjs_engine):
    assert eval_js(qjs_engine, 'indentListLine("- Hyphen")').toString() == "  - Hyphen"
    assert eval_js(qjs_engine, 'indentListLine("  - Hyphen")').toString() == "    - Hyphen"
    assert eval_js(qjs_engine, 'indentListLine("1. One")').toString() == "  1. One"
    assert eval_js(qjs_engine, 'indentListLine("  1. One")').toString() == "    1. One"

def test_indentListLine_empty_and_plain(qjs_engine):
    assert eval_js(qjs_engine, 'indentListLine("")').toString() == ""
    assert eval_js(qjs_engine, 'indentListLine("   ")').toString() == "   "
    assert eval_js(qjs_engine, 'indentListLine("Plain text")').toString() == "  Plain text"

# ---------------------------------------------------------------------------
# 2. outdentListLine
# ---------------------------------------------------------------------------

def test_outdentListLine_asterisk(qjs_engine):
    assert eval_js(qjs_engine, 'outdentListLine("***** Item")').toString() == "**** Item"
    assert eval_js(qjs_engine, 'outdentListLine("**** Item")').toString() == "*** Item"
    assert eval_js(qjs_engine, 'outdentListLine("*** Item")').toString() == "** Item"
    assert eval_js(qjs_engine, 'outdentListLine("** Item")').toString() == "* Item"
    assert eval_js(qjs_engine, 'outdentListLine("* Item")').toString() == "* Item"  # clamped

def test_outdentListLine_task(qjs_engine):
    assert eval_js(qjs_engine, 'outdentListLine("*** [ ] Task")').toString() == "** [ ] Task"
    assert eval_js(qjs_engine, 'outdentListLine("** [x] Done")').toString() == "* [x] Done"
    assert eval_js(qjs_engine, 'outdentListLine("* [ ] Task")').toString() == "* [ ] Task"  # clamped

def test_outdentListLine_dot(qjs_engine):
    assert eval_js(qjs_engine, 'outdentListLine("..... Fifth")').toString() == ".... Fifth"
    assert eval_js(qjs_engine, 'outdentListLine(".. Second")').toString() == ". Second"
    assert eval_js(qjs_engine, 'outdentListLine(". First")').toString() == ". First"  # clamped

def test_outdentListLine_hyphen_and_number(qjs_engine):
    assert eval_js(qjs_engine, 'outdentListLine("    - Hyphen")').toString() == "  - Hyphen"
    assert eval_js(qjs_engine, 'outdentListLine("  - Hyphen")').toString() == "- Hyphen"
    assert eval_js(qjs_engine, 'outdentListLine("- Hyphen")').toString() == "- Hyphen"  # clamped
    assert eval_js(qjs_engine, 'outdentListLine("    1. One")').toString() == "  1. One"
    assert eval_js(qjs_engine, 'outdentListLine("  1. One")').toString() == "1. One"
    assert eval_js(qjs_engine, 'outdentListLine("1. One")').toString() == "1. One"  # clamped

def test_outdentListLine_empty_and_plain(qjs_engine):
    assert eval_js(qjs_engine, 'outdentListLine("")').toString() == ""
    assert eval_js(qjs_engine, 'outdentListLine("Plain text")').toString() == "Plain text"
    assert eval_js(qjs_engine, 'outdentListLine("  Plain text")').toString() == "Plain text"

# ---------------------------------------------------------------------------
# 3. changeListLevel
# ---------------------------------------------------------------------------

def test_changeListLevel_single_line_indent(qjs_engine):
    js = 'JSON.stringify(changeListLevel("* Item 1", 4, 4, 4, 1))'
    res = json.loads(eval_js(qjs_engine, js).toString())
    assert res["text"] == "** Item 1"
    assert res["cursorPosition"] == 5
    assert res["selectionStart"] == 5
    assert res["selectionEnd"] == 5

def test_changeListLevel_single_line_outdent(qjs_engine):
    js = 'JSON.stringify(changeListLevel("*** Item 1", 5, 5, 5, -1))'
    res = json.loads(eval_js(qjs_engine, js).toString())
    assert res["text"] == "** Item 1"
    assert res["cursorPosition"] == 4
    assert res["selectionStart"] == 4
    assert res["selectionEnd"] == 4

def test_changeListLevel_single_line_clamped(qjs_engine):
    js = 'JSON.stringify(changeListLevel("* Root item", 4, 4, 4, -1))'
    res = json.loads(eval_js(qjs_engine, js).toString())
    assert res["text"] == "* Root item"
    assert res["cursorPosition"] == 4

def test_changeListLevel_multiline_selection(qjs_engine):
    text = "* Item A\n* Item B\n* Item C"
    # Select from line 0 index 2 to line 2 index 4
    selStart = 2
    selEnd = len("* Item A\n* Item B\n* It")
    cursorPos = selEnd
    js = f'JSON.stringify(changeListLevel({json.dumps(text)}, {selStart}, {selEnd}, {cursorPos}, 1))'
    res = json.loads(eval_js(qjs_engine, js).toString())
    assert res["text"] == "** Item A\n** Item B\n** Item C"
    # Line 0 shifted by +1 (at selStart 2 -> 3)
    assert res["selectionStart"] == 3
    # Line 0 (+1), Line 1 (+1), Line 2 (+1) -> cumulative +3 for selEnd/cursorPos
    assert res["selectionEnd"] == selEnd + 3
    assert res["cursorPosition"] == cursorPos + 3

def test_changeListLevel_mixed_and_empty_lines(qjs_engine):
    text = "* Bullet\n\n. Dot\n- Hyphen"
    js = f'JSON.stringify(changeListLevel({json.dumps(text)}, 0, {len(text)}, 0, 1))'
    res = json.loads(eval_js(qjs_engine, js).toString())
    assert res["text"] == "** Bullet\n\n.. Dot\n  - Hyphen"

# ---------------------------------------------------------------------------
# 4. moveLines
# ---------------------------------------------------------------------------

def test_moveLines_single_line_up(qjs_engine):
    text = "Line 1\nLine 2\nLine 3"
    # Cursor on Line 2 (index 9)
    cursorPos = 9
    js = f'JSON.stringify(moveLines({json.dumps(text)}, {cursorPos}, {cursorPos}, {cursorPos}, -1))'
    res = json.loads(eval_js(qjs_engine, js).toString())
    assert res["text"] == "Line 2\nLine 1\nLine 3"
    assert res["cursorPosition"] == 2  # 9 - (6 + 1) = 2

def test_moveLines_single_line_down(qjs_engine):
    text = "Line 1\nLine 2\nLine 3"
    # Cursor on Line 2 (index 9)
    cursorPos = 9
    js = f'JSON.stringify(moveLines({json.dumps(text)}, {cursorPos}, {cursorPos}, {cursorPos}, 1))'
    res = json.loads(eval_js(qjs_engine, js).toString())
    assert res["text"] == "Line 1\nLine 3\nLine 2"
    assert res["cursorPosition"] == 16  # 9 + (6 + 1) = 16

def test_moveLines_boundaries(qjs_engine):
    text = "Line 1\nLine 2\nLine 3"
    # Move Up on Line 1
    js_up = f'JSON.stringify(moveLines({json.dumps(text)}, 2, 2, 2, -1))'
    res_up = json.loads(eval_js(qjs_engine, js_up).toString())
    assert res_up["text"] == text
    assert res_up["cursorPosition"] == 2

    # Move Down on Line 3
    cursorPos3 = len(text) - 2
    js_down = f'JSON.stringify(moveLines({json.dumps(text)}, {cursorPos3}, {cursorPos3}, {cursorPos3}, 1))'
    res_down = json.loads(eval_js(qjs_engine, js_down).toString())
    assert res_down["text"] == text
    assert res_down["cursorPosition"] == cursorPos3

def test_moveLines_multiline_selection(qjs_engine):
    text = "A\nB\nC\nD"
    # Select B and C (indices 2 to 5)
    selStart = 2
    selEnd = 5
    cursorPos = 5
    js = f'JSON.stringify(moveLines({json.dumps(text)}, {selStart}, {selEnd}, {cursorPos}, 1))'
    res = json.loads(eval_js(qjs_engine, js).toString())
    assert res["text"] == "A\nD\nB\nC"
    # D length is 1, plus newline = 2
    assert res["selectionStart"] == 4
    assert res["selectionEnd"] == 7
    assert res["cursorPosition"] == 7

def test_moveLines_varying_line_lengths(qjs_engine):
    text = "Short\nVery long line here\nEnd"
    # Cursor on "Very long line here" at col 5 (index 6 + 5 = 11)
    cursorPos = 11
    js = f'JSON.stringify(moveLines({json.dumps(text)}, {cursorPos}, {cursorPos}, {cursorPos}, -1))'
    res = json.loads(eval_js(qjs_engine, js).toString())
    assert res["text"] == "Very long line here\nShort\nEnd"
    assert res["cursorPosition"] == 5

# ---------------------------------------------------------------------------
# 5. handleSmartEnter
# ---------------------------------------------------------------------------

def test_handleSmartEnter_non_empty_bullet(qjs_engine):
    text = "* Hello"
    cursorPos = len(text)
    js = f'JSON.stringify(handleSmartEnter({json.dumps(text)}, {cursorPos}))'
    res = json.loads(eval_js(qjs_engine, js).toString())
    assert res["handled"] is True
    assert res["text"] == "* Hello\n* "
    assert res["cursorPosition"] == len("* Hello\n* ")

def test_handleSmartEnter_non_empty_task(qjs_engine):
    text = "** [x] Done task"
    cursorPos = len(text)
    js = f'JSON.stringify(handleSmartEnter({json.dumps(text)}, {cursorPos}))'
    res = json.loads(eval_js(qjs_engine, js).toString())
    assert res["handled"] is True
    assert res["text"] == "** [x] Done task\n** [ ] "
    assert res["cursorPosition"] == len("** [x] Done task\n** [ ] ")

def test_handleSmartEnter_non_empty_dot(qjs_engine):
    text = ".. Step"
    cursorPos = len(text)
    js = f'JSON.stringify(handleSmartEnter({json.dumps(text)}, {cursorPos}))'
    res = json.loads(eval_js(qjs_engine, js).toString())
    assert res["handled"] is True
    assert res["text"] == ".. Step\n.. "
    assert res["cursorPosition"] == len(".. Step\n.. ")

def test_handleSmartEnter_non_empty_numbered(qjs_engine):
    text = "1. First"
    cursorPos = len(text)
    js = f'JSON.stringify(handleSmartEnter({json.dumps(text)}, {cursorPos}))'
    res = json.loads(eval_js(qjs_engine, js).toString())
    assert res["handled"] is True
    assert res["text"] == "1. First\n2. "
    assert res["cursorPosition"] == len("1. First\n2. ")

def test_handleSmartEnter_non_empty_hyphen(qjs_engine):
    text = "  - Hyphen item"
    cursorPos = len(text)
    js = f'JSON.stringify(handleSmartEnter({json.dumps(text)}, {cursorPos}))'
    res = json.loads(eval_js(qjs_engine, js).toString())
    assert res["handled"] is True
    assert res["text"] == "  - Hyphen item\n  - "
    assert res["cursorPosition"] == len("  - Hyphen item\n  - ")

def test_handleSmartEnter_empty_termination(qjs_engine):
    # Empty bullet termination
    text = "* "
    cursorPos = 2
    js = f'JSON.stringify(handleSmartEnter({json.dumps(text)}, {cursorPos}))'
    res = json.loads(eval_js(qjs_engine, js).toString())
    assert res["handled"] is True
    assert res["text"] == ""
    assert res["cursorPosition"] == 0

    # Nested empty bullet outdents to single bullet
    text_nested = "** "
    cursorPos_nested = 3
    js_nested = f'JSON.stringify(handleSmartEnter({json.dumps(text_nested)}, {cursorPos_nested}))'
    res_nested = json.loads(eval_js(qjs_engine, js_nested).toString())
    assert res_nested["handled"] is True
    assert res_nested["text"] == "* "
    assert res_nested["cursorPosition"] == 2

    # Empty numbered item termination
    text_num = "1. "
    cursorPos_num = 3
    js_num = f'JSON.stringify(handleSmartEnter({json.dumps(text_num)}, {cursorPos_num}))'
    res_num = json.loads(eval_js(qjs_engine, js_num).toString())
    assert res_num["handled"] is True
    assert res_num["text"] == ""
    assert res_num["cursorPosition"] == 0

def test_handleSmartEnter_middle_of_line(qjs_engine):
    text = "* Hello World"
    # Cursor right after "Hello" (col 7)
    cursorPos = 7
    js = f'JSON.stringify(handleSmartEnter({json.dumps(text)}, {cursorPos}))'
    res = json.loads(eval_js(qjs_engine, js).toString())
    assert res["handled"] is True
    assert res["text"] == "* Hello\n*  World"
    assert res["cursorPosition"] == len("* Hello\n* ")

def test_handleSmartEnter_non_list(qjs_engine):
    text = "Just normal text"
    cursorPos = len(text)
    js = f'JSON.stringify(handleSmartEnter({json.dumps(text)}, {cursorPos}))'
    res = json.loads(eval_js(qjs_engine, js).toString())
    assert res["handled"] is False

# ---------------------------------------------------------------------------
# 6. handleEditorKeyPress
# ---------------------------------------------------------------------------

def test_handleEditorKeyPress_tab_indent(qjs_engine):
    js = """
    var textArea = { text: "* Hello", cursorPosition: 4, selectionStart: 4, selectionEnd: 4 };
    var event = { key: 0x01000001 /* Tab */, modifiers: 0, accepted: false };
    var handled = handleEditorKeyPress(event, textArea);
    [handled, event.accepted, textArea.text, textArea.cursorPosition]
    """
    res = eval_js(qjs_engine, js)
    assert res.property(0).toBool() is True
    assert res.property(1).toBool() is True
    assert res.property(2).toString() == "** Hello"
    assert res.property(3).toInt() == 5

def test_handleEditorKeyPress_shift_tab_outdent(qjs_engine):
    js = """
    var textArea = { text: "** Hello", cursorPosition: 5, selectionStart: 5, selectionEnd: 5 };
    var event = { key: 0x01000001 /* Tab */, modifiers: 0x02000000 /* Shift */, accepted: false };
    var handled = handleEditorKeyPress(event, textArea);
    [handled, event.accepted, textArea.text, textArea.cursorPosition]
    """
    res = eval_js(qjs_engine, js)
    assert res.property(0).toBool() is True
    assert res.property(1).toBool() is True
    assert res.property(2).toString() == "* Hello"
    assert res.property(3).toInt() == 4

def test_handleEditorKeyPress_alt_up_down(qjs_engine):
    js = """
    var textArea = { text: "Line 1\\nLine 2\\nLine 3", cursorPosition: 9, selectionStart: 9, selectionEnd: 9 };
    var eventUp = { key: 0x01000013 /* Key_Up */, modifiers: 0x08000000 /* Alt */, accepted: false };
    var handledUp = handleEditorKeyPress(eventUp, textArea);
    var textAfterUp = textArea.text;
    var posAfterUp = textArea.cursorPosition;

    var eventDown = { key: 0x01000015 /* Key_Down */, modifiers: 0x08000000 /* Alt */, accepted: false };
    var handledDown = handleEditorKeyPress(eventDown, textArea);
    var textAfterDown = textArea.text;

    [handledUp, eventUp.accepted, textAfterUp, posAfterUp, handledDown, eventDown.accepted, textAfterDown]
    """
    res = eval_js(qjs_engine, js)
    assert res.property(0).toBool() is True
    assert res.property(1).toBool() is True
    assert res.property(2).toString() == "Line 2\nLine 1\nLine 3"
    assert res.property(3).toInt() == 2
    assert res.property(4).toBool() is True
    assert res.property(5).toBool() is True
    assert res.property(6).toString() == "Line 1\nLine 2\nLine 3"

def test_handleEditorKeyPress_smart_enter(qjs_engine):
    js = """
    var textArea = { text: "* Hello", cursorPosition: 7, selectionStart: 7, selectionEnd: 7 };
    var event = { key: 0x01000004 /* Key_Return */, modifiers: 0, accepted: false };
    var handled = handleEditorKeyPress(event, textArea);
    [handled, event.accepted, textArea.text, textArea.cursorPosition]
    """
    res = eval_js(qjs_engine, js)
    assert res.property(0).toBool() is True
    assert res.property(1).toBool() is True
    assert res.property(2).toString() == "* Hello\n* "
    assert res.property(3).toInt() == len("* Hello\n* ")

def test_handleEditorKeyPress_normal_key_not_handled(qjs_engine):
    js = """
    var textArea = { text: "Some text", cursorPosition: 4, selectionStart: 4, selectionEnd: 4 };
    var event = { key: 0x41 /* Key_A */, modifiers: 0, accepted: false };
    var handled = handleEditorKeyPress(event, textArea);
    [handled, event.accepted]
    """
    res = eval_js(qjs_engine, js)
    assert res.property(0).toBool() is False
    assert res.property(1).toBool() is False
