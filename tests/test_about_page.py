import os


def test_about_page_exists_and_contains_attributions():
    path = "notesplus-sailfish/qml/pages/AboutPage.qml"
    assert os.path.exists(path), f"File {path} does not exist"
    with open(path, "r", encoding="utf-8") as f:
        content = f.read()

    # Verify headers and title
    assert 'title: qsTr("About Notes Plus")' in content
    assert "Notes Plus v" in content
    assert 'qsTr("Inspirations")' in content
    assert 'qsTr("Third-Party Libraries & Attributions")' in content

    # Verify inspirations
    assert "Jolla Notes" in content
    assert "SpeechNote" in content

    # Verify third party libraries
    libs = [
        "whisper.cpp",
        "Vue.js",
        "svgbob",
        "syntect",
        "SQLite",
        "rustls",
        "tiny_http",
        "ureq",
    ]
    for lib in libs:
        assert lib in content, f"Library {lib} attribution missing from AboutPage.qml"


def test_main_page_links_to_about_page():
    path = "notesplus-sailfish/qml/pages/MainPage.qml"
    assert os.path.exists(path), f"File {path} does not exist"
    with open(path, "r", encoding="utf-8") as f:
        content = f.read()
    assert "AboutPage.qml" in content, f"{path} missing link to AboutPage.qml"


def test_release_stamp_script_updates_about_page():
    path = "scripts/release.d/23-stamp-version-in-qml.sh"
    assert os.path.exists(path), f"File {path} does not exist"
    with open(path, "r", encoding="utf-8") as f:
        content = f.read()

    assert "AboutPage.qml" in content
