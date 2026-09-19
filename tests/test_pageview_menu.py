import os
import re


def test_pageview_pulldown_menu_items():
    path = "notesplus-sailfish/qml/pages/PageView.qml"
    assert os.path.exists(path), f"File {path} does not exist"
    with open(path, "r", encoding="utf-8") as f:
        content = f.read()

    # Extract PullDownMenu content
    match = re.search(r"PullDownMenu\s*\{([\s\S]*?)\n\s*header:", content)
    assert match is not None, "PullDownMenu block not found in PageView.qml"
    menu_content = match.group(1)

    # Verify "Delete Page" and "Move to Group" have been removed from the top menu
    assert "Delete Page" not in menu_content
    assert "Move to Group" not in menu_content
    assert "bridge.delete_page" not in menu_content
    assert "MovePageDialog.qml" not in menu_content

    # Verify retained top menu actions
    assert 'text: qsTr("Find in Page")' in menu_content
    assert 'text: qsTr("Ask AI Assistant")' in menu_content
    assert 'text: qsTr("Edit Source")' in menu_content
    assert 'text: qsTr("Copy Page URL")' in menu_content
    assert 'text: qsTr("Export as PDF")' in menu_content
    assert 'text: qsTr("Share as PDF")' in menu_content


def test_notecard_preview_tile_menu_retains_move_and_delete():
    path = "notesplus-sailfish/qml/components/NoteCard.qml"
    assert os.path.exists(path), f"File {path} does not exist"
    with open(path, "r", encoding="utf-8") as f:
        content = f.read()

    # Verify NoteCard context menu still contains Move and Delete
    assert 'text: qsTr("Move")' in content
    assert "MovePageDialog.qml" in content
    assert "bridge.move_page_to_group" in content

    assert 'text: qsTr("Delete")' in content
    assert "bridge.delete_page(_fullPath)" in content
