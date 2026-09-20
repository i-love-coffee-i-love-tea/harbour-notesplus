"""End-to-end tests for notes persistence across simulated app restarts and migrations."""
import os
import sqlite3
import shutil
import pytest
from pathlib import Path


def create_mock_db(db_path: Path):
    """Initializes SQLite schema matching notesplus-core."""
    conn = sqlite3.connect(db_path)
    cursor = conn.cursor()
    cursor.execute("""
        CREATE TABLE IF NOT EXISTS pages (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            filename TEXT NOT NULL UNIQUE,
            title TEXT NOT NULL,
            mtime INTEGER NOT NULL DEFAULT 0,
            color TEXT,
            group_id INTEGER
        );
    """)
    cursor.execute("""
        CREATE TABLE IF NOT EXISTS groups (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            path TEXT NOT NULL UNIQUE,
            name TEXT NOT NULL
        );
    """)
    cursor.execute("""
        CREATE VIRTUAL TABLE IF NOT EXISTS pages_fts USING fts5(
            title,
            body,
            content='pages',
            content_rowid='id'
        );
    """)
    conn.commit()
    return conn


def sync_pages(conn: sqlite3.Connection, notes_dir: Path):
    """Simulates non-destructive sync_and_index_pages."""
    cursor = conn.cursor()
    for root, dirs, files in os.walk(notes_dir):
        for f in files:
            if not f.endswith(".adoc"):
                continue
            f_path = Path(root) / f
            rel_path = f_path.relative_to(notes_dir).as_posix()
            mtime = int(f_path.stat().st_mtime)

            # Read first line as title
            with open(f_path, "r", encoding="utf-8") as fp:
                first_line = fp.readline().strip()
                title = first_line.lstrip("= ").strip() if first_line.startswith("=") else f

            cursor.execute("SELECT id, mtime, color FROM pages WHERE filename = ?", (rel_path,))
            row = cursor.fetchone()
            if row is None:
                cursor.execute(
                    "INSERT INTO pages (filename, title, mtime, color) VALUES (?, ?, ?, '')",
                    (rel_path, title, mtime),
                )
            else:
                existing_mtime = row[1]
                if mtime > existing_mtime:
                    # Update without wiping color
                    cursor.execute(
                        "UPDATE pages SET title = ?, mtime = ? WHERE id = ?",
                        (title, mtime, row[0]),
                    )

    conn.commit()


def test_session_restart_persistence(tmp_path):
    """Verifies that notes and journal entries persist across simulated app closures/restarts."""
    data_dir = tmp_path / ".local" / "share" / "org.gobuki" / "harbour-notesplus"
    notes_dir = tmp_path / "Documents" / "Notes Plus"
    data_dir.mkdir(parents=True)
    notes_dir.mkdir(parents=True)

    db_path = data_dir / "notesplus.db"

    # --- SESSION 1: Create Note and Journal ---
    conn1 = create_mock_db(db_path)

    # 1. User creates a note
    note_file = notes_dir / "ProjectPlan.adoc"
    note_file.write_text("= Project Plan\n\nRelease Notes Plus v0.2.2 with persistent storage.\n", encoding="utf-8")

    # 2. User writes a journal entry
    journal_file = notes_dir / "journal.adoc"
    journal_file.write_text("= Journal\n\n* 2026-09-20: Implemented Sailjail-safe storage.\n", encoding="utf-8")

    sync_pages(conn1, notes_dir)

    # 3. User sets custom color on ProjectPlan
    cursor1 = conn1.cursor()
    cursor1.execute("UPDATE pages SET color = '#2ecc71' WHERE filename = 'ProjectPlan.adoc'")
    conn1.commit()

    # Simulate app close (disconnect and destroy conn1)
    conn1.close()
    del conn1

    # --- SESSION 2: App Restart ---
    # Reopen database as a new app session would
    conn2 = sqlite3.connect(db_path)
    cursor2 = conn2.cursor()

    # Non-destructive startup sync
    sync_pages(conn2, notes_dir)

    # Verify note file still exists on disk
    assert note_file.exists()
    assert "= Project Plan" in note_file.read_text(encoding="utf-8")

    # Verify journal file still exists on disk
    assert journal_file.exists()
    assert "Sailjail-safe storage" in journal_file.read_text(encoding="utf-8")

    # Verify metadata and color are preserved (NOT wiped by destructive re-indexing)
    cursor2.execute("SELECT filename, title, color FROM pages WHERE filename = 'ProjectPlan.adoc'")
    row = cursor2.fetchone()
    assert row is not None
    assert row[0] == "ProjectPlan.adoc"
    assert row[1] == "Project Plan"
    assert row[2] == "#2ecc71", "Custom color must be preserved across app restarts"

    cursor2.execute("SELECT filename, title FROM pages WHERE filename = 'journal.adoc'")
    j_row = cursor2.fetchone()
    assert j_row is not None
    assert j_row[1] == "Journal"

    conn2.close()


def test_runtime_migration_and_restart(tmp_path):
    """Verifies that migrating to a custom storage path copies notes and preserves data across restarts."""
    data_dir = tmp_path / ".local" / "share" / "org.gobuki" / "harbour-notesplus"
    original_notes_dir = tmp_path / "Documents" / "Notes Plus"
    custom_notes_dir = tmp_path / "CustomSDCard" / "MyNotes"

    data_dir.mkdir(parents=True)
    original_notes_dir.mkdir(parents=True)

    db_path = data_dir / "notesplus.db"
    conn = create_mock_db(db_path)

    # Create initial files
    (original_notes_dir / "Architecture.adoc").write_text("= Architecture\nADR-002 and ADR-010.\n", encoding="utf-8")
    (original_notes_dir / "journal.adoc").write_text("= Journal\nSession started.\n", encoding="utf-8")

    sync_pages(conn, original_notes_dir)

    # Perform migration to custom_notes_dir
    custom_notes_dir.mkdir(parents=True)
    for root, dirs, files in os.walk(original_notes_dir):
        for f in files:
            rel = Path(root).relative_to(original_notes_dir)
            dst_dir = custom_notes_dir / rel
            dst_dir.mkdir(parents=True, exist_ok=True)
            shutil.copy2(Path(root) / f, dst_dir / f)

    # Resync database with new location
    sync_pages(conn, custom_notes_dir)

    conn.close()

    # --- Verify in a fresh session pointing to custom_notes_dir ---
    conn_new_session = sqlite3.connect(db_path)
    sync_pages(conn_new_session, custom_notes_dir)

    # Verify both notes exist in new custom location
    assert (custom_notes_dir / "Architecture.adoc").exists()
    assert (custom_notes_dir / "journal.adoc").exists()
    assert "= Architecture" in (custom_notes_dir / "Architecture.adoc").read_text(encoding="utf-8")

    # Verify original files were NOT deleted (safety guarantee)
    assert (original_notes_dir / "Architecture.adoc").exists()
    assert (original_notes_dir / "journal.adoc").exists()

    conn_new_session.close()
