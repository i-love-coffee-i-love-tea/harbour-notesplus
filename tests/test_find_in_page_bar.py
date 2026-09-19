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
    return engine

def test_counterText_formatting(qjs_engine):
    js = """
    function formatCounter(searchTerm, currentMatchIndex, totalMatches) {
        if (!searchTerm || searchTerm.trim().length === 0) return "";
        if (totalMatches === 0) return "0 / 0";
        var currentNumber = (currentMatchIndex >= 0 ? currentMatchIndex : 0) + 1;
        return currentNumber + " / " + totalMatches;
    }
    [
        formatCounter("", 0, 0),
        formatCounter("   ", 0, 0),
        formatCounter("test", -1, 0),
        formatCounter("test", 0, 5),
        formatCounter("test", 2, 12),
        formatCounter("test", 3, 4)
    ]
    """
    res = qjs_engine.evaluate(js)
    assert res.property(0).toString() == ""
    assert res.property(1).toString() == ""
    assert res.property(2).toString() == "0 / 0"
    assert res.property(3).toString() == "1 / 5"
    assert res.property(4).toString() == "3 / 12"
    assert res.property(5).toString() == "4 / 4"

def test_buttonEnabledStates(qjs_engine):
    js = """
    function getButtonStates(totalMatches) {
        var enabled = totalMatches > 0;
        return {
            prevEnabled: enabled,
            nextEnabled: enabled,
            opacity: enabled ? 1.0 : 0.3
        };
    }
    var s0 = getButtonStates(0);
    var s1 = getButtonStates(1);
    var s5 = getButtonStates(5);
    [s0.prevEnabled, s0.opacity, s1.prevEnabled, s1.opacity, s5.nextEnabled, s5.opacity]
    """
    res = qjs_engine.evaluate(js)
    assert res.property(0).toBool() is False
    assert abs(res.property(1).toNumber() - 0.3) < 0.01
    assert res.property(2).toBool() is True
    assert abs(res.property(3).toNumber() - 1.0) < 0.01
    assert res.property(4).toBool() is True
    assert abs(res.property(5).toNumber() - 1.0) < 0.01

def test_wrapAroundNavigation(qjs_engine):
    js = """
    function nextMatchIndex(current, total) {
        if (total <= 0) return -1;
        return (current + 1) % total;
    }
    function prevMatchIndex(current, total) {
        if (total <= 0) return -1;
        return (current - 1 + total) % total;
    }
    [
        nextMatchIndex(0, 4),    // 1
        nextMatchIndex(3, 4),    // 0 (wrap around)
        prevMatchIndex(0, 4),    // 3 (wrap around)
        prevMatchIndex(2, 4),    // 1
        nextMatchIndex(0, 1),    // 0
        prevMatchIndex(0, 1)     // 0
    ]
    """
    res = qjs_engine.evaluate(js)
    assert res.property(0).toInt() == 1
    assert res.property(1).toInt() == 0
    assert res.property(2).toInt() == 3
    assert res.property(3).toInt() == 1
    assert res.property(4).toInt() == 0
    assert res.property(5).toInt() == 0

def test_findInPageBar_qml_exists_and_contains_contracts():
    import os
    path = "notesplus-sailfish/qml/components/editor/FindInPageBar.qml"
    assert os.path.exists(path), f"File {path} does not exist"
    with open(path, "r", encoding="utf-8") as f:
        content = f.read()

    # Verify key properties and signals exist
    assert "property string searchTerm" in content or "property alias text" in content or "property alias searchTerm" in content
    assert "property int currentMatchIndex" in content
    assert "property int totalMatches" in content
    assert "signal nextClicked" in content
    assert "signal previousClicked" in content
    assert "signal closeClicked" in content
    assert "SearchField" in content
    assert "icon-m-up" in content
    assert "icon-m-down" in content
    assert "icon-m-close" in content
