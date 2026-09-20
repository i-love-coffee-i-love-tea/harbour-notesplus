import ctypes
import json
import os
import sqlite3
import tempfile
from pathlib import Path


def test_sort_notes_dialog_structure():
    """Verify SortNotesDialog.qml has all required properties, header, and sorting ComboBox."""
    path = Path("notesplus-sailfish/qml/dialogs/SortNotesDialog.qml")
    assert path.exists(), "SortNotesDialog.qml must exist in qml/dialogs/"
    content = path.read_text(encoding="utf-8")

    # Property definitions
    assert "property string groupPath" in content
    assert "property string groupName" in content
    assert "property string currentSort" in content
    assert "property string selectedSort" in content

    # Header and title
    assert 'title: qsTr("Sort Notes")' in content
    assert 'acceptText: qsTr("Save")' in content
    assert 'cancelText: qsTr("Cancel")' in content

    # Options in ComboBox
    assert "ComboBox {" in content
    assert 'MenuItem { text: qsTr("Newest first") }' in content
    assert 'MenuItem { text: qsTr("By name") }' in content
    assert 'currentIndex: currentSort === "name" ? 1 : 0' in content


def test_note_group_section_sort_menu_item():
    """Verify NoteGroupSection.qml opens SortNotesDialog when Sort Notes is clicked."""
    path = Path("notesplus-sailfish/qml/components/common/NoteGroupSection.qml")
    assert path.exists()
    content = path.read_text(encoding="utf-8")

    # Should have a generic 'Sort Notes' menu item instead of toggling directly
    assert 'text: qsTr("Sort Notes")' in content
    assert "SortNotesDialog.qml" in content
    assert "pageStack.push" in content
    assert "groupPath: groupPath" in content
    assert "groupName: displayName" in content
    assert "currentSort: noteSort" in content
    assert "bridge.set_group_note_sort(groupPath, sortDlg.selectedSort)" in content


def test_group_manager_sort_mappings():
    """Verify GroupManager.cpp maps 'name' to 1 and returns 'name' for sort 1."""
    path = Path("notesplus-sailfish/src/bridge/GroupManager.cpp")
    assert path.exists()
    content = path.read_text(encoding="utf-8")

    assert 'note_sort == QLatin1String("name")' in content
    assert 'case 1:  return QStringLiteral("name");' in content
    assert 'default: return QStringLiteral("newest");' in content


def test_core_group_note_sort_integration():
    """Verify end-to-end note sorting via libnotesplus_core FFI and build_group_tree."""
    core = ctypes.CDLL("./target/debug/libnotesplus_core.so")

    core.notes_core_db_open.argtypes = [ctypes.c_char_p]
    core.notes_core_db_open.restype = ctypes.c_void_p

    core.notes_core_db_close.argtypes = [ctypes.c_void_p]

    core.notes_core_group_create.argtypes = [
        ctypes.c_void_p, ctypes.c_char_p, ctypes.c_char_p, ctypes.c_char_p
    ]
    core.notes_core_group_create.restype = ctypes.c_int32

    core.notes_core_group_set_note_sort.argtypes = [
        ctypes.c_void_p, ctypes.c_char_p, ctypes.c_int32
    ]
    core.notes_core_group_set_note_sort.restype = ctypes.c_int32

    core.notes_core_group_get_note_sort.argtypes = [
        ctypes.c_void_p, ctypes.c_char_p
    ]
    core.notes_core_group_get_note_sort.restype = ctypes.c_int32

    core.notes_core_sync_index.argtypes = [ctypes.c_void_p, ctypes.c_char_p]
    core.notes_core_sync_index.restype = ctypes.c_int32

    core.notes_core_build_group_tree_json.argtypes = [
        ctypes.c_void_p, ctypes.c_char_p, ctypes.c_int32, ctypes.c_int32,
        ctypes.c_char_p, ctypes.c_char_p
    ]
    core.notes_core_build_group_tree_json.restype = ctypes.c_void_p

    core.notes_core_free_string.argtypes = [ctypes.c_void_p]

    with tempfile.TemporaryDirectory() as tmpdir:
        raw_db_path = os.path.join(tmpdir, "test.db")
        db_path = raw_db_path.encode("utf-8")
        notes_dir = os.path.join(tmpdir, "notes")
        os.makedirs(notes_dir, exist_ok=True)
        notes_dir_b = notes_dir.encode("utf-8")

        conn = core.notes_core_db_open(db_path)
        assert conn is not None

        try:
            # Create group "Project"
            rc = core.notes_core_group_create(conn, notes_dir_b, b"", b"Project")
            assert rc == 0

            # Default sort order should be 0 (newest)
            sort_order = core.notes_core_group_get_note_sort(conn, b"Project")
            assert sort_order == 0

            # Create group folder
            proj_dir = os.path.join(notes_dir, "Project")
            os.makedirs(proj_dir, exist_ok=True)

            b_path = os.path.join(proj_dir, "b_first.adoc")
            with open(b_path, "w", encoding="utf-8") as f:
                f.write("= Beta Note\n\nBeta content.\n")

            a_path = os.path.join(proj_dir, "a_second.adoc")
            with open(a_path, "w", encoding="utf-8") as f:
                f.write("= Alpha Note\n\nAlpha content.\n")

            z_path = os.path.join(proj_dir, "z_third.adoc")
            with open(z_path, "w", encoding="utf-8") as f:
                f.write("= Zebra Note\n\nZebra content.\n")

            rc = core.notes_core_sync_index(conn, notes_dir_b)
            assert rc == 0

            # Set distinct updated_at timestamps:
            # b: oldest (2026-01-01), a: middle (2026-01-02), z: newest (2026-01-03)
            sconn = sqlite3.connect(raw_db_path)
            sconn.execute("UPDATE pages SET updated_at = '2026-01-01T00:00:00Z' WHERE filename = 'b_first.adoc'")
            sconn.execute("UPDATE pages SET updated_at = '2026-01-02T00:00:00Z' WHERE filename = 'a_second.adoc'")
            sconn.execute("UPDATE pages SET updated_at = '2026-01-03T00:00:00Z' WHERE filename = 'z_third.adoc'")
            sconn.commit()
            sconn.close()

            # 1. Test "newest" sorting (sort_order = 0):
            # Order must be Zebra (newest), Alpha, Beta (oldest)
            ptr = core.notes_core_build_group_tree_json(
                conn, notes_dir_b, 1, 1, None, None
            )
            assert ptr is not None
            tree_data = json.loads(ctypes.string_at(ptr).decode("utf-8"))
            core.notes_core_free_string(ptr)

            group_node = next(g for g in tree_data if g.get("path") == "Project")
            assert group_node["note_sort"] == "newest"
            note_titles = [n["title"] for n in group_node["pages"]]
            assert note_titles == ["Zebra Note", "Alpha Note", "Beta Note"]

            # 2. Change sort order to 1 ("name"):
            rc = core.notes_core_group_set_note_sort(conn, b"Project", 1)
            assert rc == 0
            assert core.notes_core_group_get_note_sort(conn, b"Project") == 1

            ptr = core.notes_core_build_group_tree_json(
                conn, notes_dir_b, 1, 1, None, None
            )
            assert ptr is not None
            tree_data = json.loads(ctypes.string_at(ptr).decode("utf-8"))
            core.notes_core_free_string(ptr)

            group_node = next(g for g in tree_data if g.get("path") == "Project")
            assert group_node["note_sort"] == "name"
            note_titles = [n["title"] for n in group_node["pages"]]
            # Alphabetical: Alpha, Beta, Zebra
            assert note_titles == ["Alpha Note", "Beta Note", "Zebra Note"]

            # 3. Change back to "newest" (0):
            rc = core.notes_core_group_set_note_sort(conn, b"Project", 0)
            assert rc == 0
            assert core.notes_core_group_get_note_sort(conn, b"Project") == 0

            ptr = core.notes_core_build_group_tree_json(
                conn, notes_dir_b, 1, 1, None, None
            )
            tree_data = json.loads(ctypes.string_at(ptr).decode("utf-8"))
            core.notes_core_free_string(ptr)
            group_node = next(g for g in tree_data if g.get("path") == "Project")
            assert group_node["note_sort"] == "newest"
            note_titles = [n["title"] for n in group_node["pages"]]
            # Under "newest": Zebra, Alpha, Beta
            assert note_titles == ["Zebra Note", "Alpha Note", "Beta Note"]

        finally:
            core.notes_core_db_close(conn)
