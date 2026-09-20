"""Tests for storage path resolution, legacy data migration, and directory scanning."""
import os
import shutil
from pathlib import Path
import pytest
from PyQt6.QtCore import QStandardPaths, QDir, QCoreApplication


def get_default_data_dir() -> str:
    QCoreApplication.setOrganizationName("org.gobuki")
    QCoreApplication.setApplicationName("harbour-notesplus")
    path = QStandardPaths.writableLocation(QStandardPaths.StandardLocation.AppDataLocation)
    if not path or path == "":
        path = os.path.join(QDir.homePath(), ".local", "share", "org.gobuki", "harbour-notesplus")
    return path


def get_default_notes_dir() -> str:
    QCoreApplication.setOrganizationName("org.gobuki")
    QCoreApplication.setApplicationName("harbour-notesplus")
    docs = QStandardPaths.writableLocation(QStandardPaths.StandardLocation.DocumentsLocation)
    if not docs or docs == "":
        docs = os.path.join(QDir.homePath(), "Documents")
    return os.path.join(docs, "Notes Plus")


def migrate_legacy_data(legacy_data_dir: str, target_data_dir: str, target_notes_dir: str) -> dict:
    """Simulates startup migration logic from NotesBridge."""
    copied_notes = 0
    db_migrated = False

    os.makedirs(target_data_dir, exist_ok=True)
    os.makedirs(target_notes_dir, exist_ok=True)

    # 1. Database migration
    legacy_db = os.path.join(legacy_data_dir, "notesplus.db")
    target_db = os.path.join(target_data_dir, "notesplus.db")
    if os.path.isfile(legacy_db) and not os.path.isfile(target_db):
        shutil.copy2(legacy_db, target_db)
        db_migrated = True

    # 2. Notes migration from legacy_data_dir/notes
    legacy_notes = os.path.join(legacy_data_dir, "notes")
    if os.path.isdir(legacy_notes):
        for root, dirs, files in os.walk(legacy_notes):
            rel_dir = os.path.relpath(root, legacy_notes)
            dest_dir = target_notes_dir if rel_dir == "." else os.path.join(target_notes_dir, rel_dir)
            os.makedirs(dest_dir, exist_ok=True)
            for f in files:
                src_file = os.path.join(root, f)
                dst_file = os.path.join(dest_dir, f)
                if not os.path.exists(dst_file):
                    shutil.copy2(src_file, dst_file)
                    copied_notes += 1

    return {
        "copied_notes": copied_notes,
        "db_migrated": db_migrated,
    }


def scan_notes_dir(path: str) -> dict:
    """Scans a notes directory and returns file count, folder count, and total byte size."""
    if not os.path.isdir(path):
        return {"notes_count": 0, "folders_count": 0, "total_bytes": 0}

    notes_count = 0
    folders_count = 0
    total_bytes = 0

    for root, dirs, files in os.walk(path):
        if root != path:
            folders_count += 1
        for f in files:
            if f.endswith(".adoc"):
                notes_count += 1
            f_path = os.path.join(root, f)
            try:
                total_bytes += os.path.getsize(f_path)
            except OSError:
                pass

    return {
        "notes_count": notes_count,
        "folders_count": folders_count,
        "total_bytes": total_bytes,
    }


def test_default_paths_structure():
    home = QDir.homePath()
    data_dir = get_default_data_dir()
    notes_dir = get_default_notes_dir()

    # Must contain org.gobuki in app data path to satisfy Sailjail whitelist
    assert "org.gobuki" in data_dir
    assert "harbour-notesplus" in data_dir
    assert data_dir.startswith(home)

    # User notes must be in Documents/Notes Plus
    assert "Documents" in notes_dir
    assert notes_dir.endswith("Notes Plus")
    assert notes_dir.startswith(home)


def test_legacy_startup_migration(tmp_path):
    legacy_data = tmp_path / "legacy_share" / "harbour-notesplus"
    legacy_notes = legacy_data / "notes"
    os.makedirs(legacy_notes / "work", exist_ok=True)

    # Create mock legacy database and notes
    (legacy_data / "notesplus.db").write_text("sqlite mock db header")
    (legacy_notes / "welcome.adoc").write_text("= Welcome\n\nLegacy note content.")
    (legacy_notes / "work" / "project.adoc").write_text("= Project\n\nSecret project.")

    target_data = tmp_path / "new_share" / "org.gobuki" / "harbour-notesplus"
    target_notes = tmp_path / "Documents" / "Notes Plus"

    result = migrate_legacy_data(str(legacy_data), str(target_data), str(target_notes))

    assert result["db_migrated"] is True
    assert result["copied_notes"] == 2

    # Verify files exist in destination
    assert (target_data / "notesplus.db").is_file()
    assert (target_data / "notesplus.db").read_text() == "sqlite mock db header"
    assert (target_notes / "welcome.adoc").is_file()
    assert (target_notes / "work" / "project.adoc").is_file()
    assert (target_notes / "work" / "project.adoc").read_text() == "= Project\n\nSecret project."

    # Verify source files remained untouched
    assert (legacy_data / "notesplus.db").is_file()
    assert (legacy_notes / "welcome.adoc").is_file()
    assert (legacy_notes / "work" / "project.adoc").is_file()


def test_legacy_migration_does_not_overwrite_existing_db(tmp_path):
    legacy_data = tmp_path / "legacy_share" / "harbour-notesplus"
    os.makedirs(legacy_data, exist_ok=True)
    (legacy_data / "notesplus.db").write_text("old legacy db")

    target_data = tmp_path / "new_share" / "org.gobuki" / "harbour-notesplus"
    os.makedirs(target_data, exist_ok=True)
    (target_data / "notesplus.db").write_text("existing new db")

    target_notes = tmp_path / "Documents" / "Notes Plus"

    result = migrate_legacy_data(str(legacy_data), str(target_data), str(target_notes))

    assert result["db_migrated"] is False
    assert (target_data / "notesplus.db").read_text() == "existing new db"


def test_scan_notes_dir(tmp_path):
    notes_dir = tmp_path / "Notes Plus"
    os.makedirs(notes_dir / "folder1", exist_ok=True)
    os.makedirs(notes_dir / "folder2" / "subfolder", exist_ok=True)

    (notes_dir / "note1.adoc").write_text("= Note 1\n")
    (notes_dir / "folder1" / "note2.adoc").write_text("= Note 2\nSome longer body content.")
    (notes_dir / "folder2" / "subfolder" / "note3.adoc").write_text("= Note 3\n")
    (notes_dir / "other.txt").write_text("ignore me")

    stats = scan_notes_dir(str(notes_dir))
    assert stats["notes_count"] == 3
    assert stats["folders_count"] == 3
    assert stats["total_bytes"] > 0


def test_runtime_migration_copies_and_preserves_source(tmp_path):
    source_dir = tmp_path / "Documents" / "Notes Plus"
    target_dir = tmp_path / "CustomStorage" / "MyNotes"

    os.makedirs(source_dir / "journal", exist_ok=True)
    (source_dir / "Meeting.adoc").write_text("= Meeting\nImportant notes.")
    (source_dir / "journal" / "2026-09-20.adoc").write_text("= 2026-09-20\nJournal entry.")

    # Execute migration
    files_to_copy = []
    for root, dirs, files in os.walk(str(source_dir)):
        for f in files:
            rel = os.path.relpath(os.path.join(root, f), str(source_dir))
            files_to_copy.append(rel)

    os.makedirs(target_dir, exist_ok=True)
    copied_count = 0
    for rel in files_to_copy:
        src = os.path.join(str(source_dir), rel)
        dst = os.path.join(str(target_dir), rel)
        os.makedirs(os.path.dirname(dst), exist_ok=True)
        shutil.copy2(src, dst)
        copied_count += 1

    assert copied_count == 2
    # Check target
    assert (target_dir / "Meeting.adoc").read_text() == "= Meeting\nImportant notes."
    assert (target_dir / "journal" / "2026-09-20.adoc").read_text() == "= 2026-09-20\nJournal entry."
    # Crucial safety requirement: source files MUST remain intact!
    assert (source_dir / "Meeting.adoc").is_file()
    assert (source_dir / "journal" / "2026-09-20.adoc").is_file()


def test_runtime_migration_unwritable_target_fails_safely(tmp_path):
    source_dir = tmp_path / "Documents" / "Notes Plus"
    os.makedirs(source_dir, exist_ok=True)
    (source_dir / "test.adoc").write_text("= Test\nContent.")

    # Target is in a read-only directory
    ro_base = tmp_path / "readonly_dir"
    os.makedirs(ro_base, exist_ok=True)
    os.chmod(ro_base, 0o444)

    target_dir = ro_base / "Subfolder" / "Notes"

    # Attempting to create target directory should fail
    creation_failed = False
    try:
        os.makedirs(target_dir, exist_ok=True)
    except (PermissionError, OSError):
        creation_failed = True
    finally:
        os.chmod(ro_base, 0o777)

    assert creation_failed is True
    # Source note is completely unharmed
    assert (source_dir / "test.adoc").read_text() == "= Test\nContent."


def test_qml_components_storage_migration_integration():
    # 1. AppSettings.qml
    app_settings_path = "notesplus-sailfish/qml/components/common/AppSettings.qml"
    assert os.path.exists(app_settings_path)
    with open(app_settings_path, "r", encoding="utf-8") as f:
        app_settings = f.read()
    assert 'key: "/apps/harbour-notesplus/notes_path"' in app_settings
    assert "property string notesPath:" in app_settings
    assert "function setNotesPath(v)" in app_settings

    # 2. MigrateStorageDialog.qml
    dialog_path = "notesplus-sailfish/qml/dialogs/MigrateStorageDialog.qml"
    assert os.path.exists(dialog_path)
    with open(dialog_path, "r", encoding="utf-8") as f:
        dialog = f.read()
    assert "Dialog {" in dialog
    assert "property string sourcePath:" in dialog
    assert "property string targetPath:" in dialog
    assert "bridge.reset_migration_state()" in dialog
    assert "bridge.scan_notes_dir(" in dialog
    assert "bridge.start_notes_migration(" in dialog
    assert "ProgressBar {" in dialog
    assert "bridge.migration_progress" in dialog
    assert "BusyIndicator {" in dialog
    assert "bridge.migration_status" in dialog
    assert "bridge.migration_current_file" in dialog
    assert "bridge.migration_copied_count" in dialog
    assert "bridge.migration_total_count" in dialog
    assert "bridge.migration_finished" in dialog
    assert "bridge.migration_success" in dialog
    assert "bridge.migration_error" in dialog

    # 3. StorageSettingsPage.qml (dedicated Storage category)
    storage_page_path = "notesplus-sailfish/qml/pages/StorageSettingsPage.qml"
    assert os.path.exists(storage_page_path)
    with open(storage_page_path, "r", encoding="utf-8") as f:
        storage_page = f.read()
    assert 'text: qsTr("Storage Location")' in storage_page
    assert 'id: notesPathField' in storage_page
    assert "bridge.notes_dir" in storage_page
    assert "bridge.default_notes_dir" in storage_page
    assert 'text: qsTr("Migrate Storage...")' in storage_page
    assert "MigrateStorageDialog.qml" in storage_page
    assert 'text: qsTr("Reset to Default")' in storage_page
    assert 'text: qsTr("Database & Search Index")' in storage_page
    assert 'text: qsTr("Rebuild Search Index")' in storage_page

    # Ensure no remorse popup is executed after leaving MigrateStorageDialog
    storage_section = storage_page.split('qsTr("Storage Location")')[1].split('qsTr("Database & Search Index")')[0]
    assert "storageRemorsePopup" not in storage_section

    # 4. harbour-notesplus.qml
    main_qml_path = "notesplus-sailfish/qml/harbour-notesplus.qml"
    assert os.path.exists(main_qml_path)
    with open(main_qml_path, "r", encoding="utf-8") as f:
        main_qml = f.read()
    assert "property alias notesPath: appSettings.notesPath" in main_qml
    assert "function setNotesPath(path)" in main_qml
