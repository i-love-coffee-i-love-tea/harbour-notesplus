"""Pytest configuration for GUI tests."""
import os
import pytest
from PyQt6.QtGui import QGuiApplication

os.environ.setdefault("QT_QPA_PLATFORM", "offscreen")

_app = None


@pytest.fixture(scope="session", autouse=True)
def qapp():
    global _app
    if _app is None:
        _app = QGuiApplication.instance() or QGuiApplication(["pytest", "-platform", "offscreen"])
    return _app


def pytest_sessionfinish(session, exitstatus):
    # Avoid Qt6 C++ destructor crash during Python 3.14 shutdown
    os._exit(exitstatus)
