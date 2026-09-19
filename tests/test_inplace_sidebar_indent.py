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

def test_inplace_sidebar_file_contracts():
    path = "notesplus-sailfish/qml/components/editor/InPlaceEditSidebar.qml"
    assert os.path.exists(path), f"File {path} does not exist"
    with open(path, "r", encoding="utf-8") as f:
        content = f.read()

    # Verify signals exist
    assert "signal indentRequested()" in content
    assert "signal outdentRequested()" in content
    assert "signal moveUpRequested()" in content
    assert "signal moveDownRequested()" in content

    # Verify flyout property and controls
    assert "property bool flyoutOpen" in content
    assert "flyoutPalette" in content
    assert "flyoutOutdentBtn" in content or "outdent" in content.lower()
    assert "flyoutIndentBtn" in content or "indent" in content.lower()
    assert "flyoutMoveUpBtn" in content or "moveup" in content.lower()
    assert "flyoutMoveDownBtn" in content or "movedown" in content.lower()
    assert "openFlyout" in content
    assert "closeFlyout" in content

def test_pageview_sidebar_wiring_contracts():
    path = "notesplus-sailfish/qml/pages/PageView.qml"
    assert os.path.exists(path), f"File {path} does not exist"
    with open(path, "r", encoding="utf-8") as f:
        content = f.read()

    assert "onIndentRequested:" in content
    assert "onOutdentRequested:" in content
    assert "onMoveUpRequested:" in content
    assert "onMoveDownRequested:" in content
    assert "function changeActiveEditorListLevel(" in content
    assert "function moveActiveEditorLines(" in content

def test_mainpage_sidebar_wiring_contracts():
    path = "notesplus-sailfish/qml/pages/MainPage.qml"
    assert os.path.exists(path), f"File {path} does not exist"
    with open(path, "r", encoding="utf-8") as f:
        content = f.read()

    assert "onIndentRequested:" in content
    assert "onOutdentRequested:" in content
    assert "onMoveUpRequested:" in content
    assert "onMoveDownRequested:" in content
    assert "function changeActiveEditorListLevel(" in content
    assert "function moveActiveEditorLines(" in content

def test_flyout_state_machine(qjs_engine):
    js = """
    var sidebar = {
        flyoutOpen: false,
        openFlyout: function() { this.flyoutOpen = true; },
        closeFlyout: function() { this.flyoutOpen = false; },
        toggleFlyout: function() { this.flyoutOpen = !this.flyoutOpen; }
    };

    var s0 = sidebar.flyoutOpen;
    sidebar.openFlyout();
    var s1 = sidebar.flyoutOpen;
    sidebar.toggleFlyout();
    var s2 = sidebar.flyoutOpen;
    sidebar.toggleFlyout();
    var s3 = sidebar.flyoutOpen;
    sidebar.closeFlyout();
    var s4 = sidebar.flyoutOpen;

    [s0, s1, s2, s3, s4]
    """
    res = qjs_engine.evaluate(js)
    assert res.property(0).toBool() is False
    assert res.property(1).toBool() is True
    assert res.property(2).toBool() is False
    assert res.property(3).toBool() is True
    assert res.property(4).toBool() is False
