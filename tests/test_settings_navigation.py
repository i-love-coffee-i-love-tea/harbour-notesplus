import os
from pathlib import Path

def test_settings_page_category_navigation():
    """Verify SettingsPage.qml uses hierarchical category navigation instead of tabs."""
    path = Path("notesplus-sailfish/qml/pages/SettingsPage.qml")
    assert path.exists()
    content = path.read_text(encoding="utf-8")

    # Tabs removed
    assert "currentTab" not in content
    assert "Row {" not in content or "preferredWidth" not in content  # No tab button row

    # Categories present
    assert 'text: qsTr("Display")' in content
    assert 'text: qsTr("Web Server")' in content
    assert 'text: qsTr("Storage")' in content
    assert 'text: qsTr("AI Assistant")' in content
    assert 'text: qsTr("Speech Recognition")' in content

    # Category icons present and valid
    assert 'source: "image://theme/icon-m-edit"' in content
    assert 'source: "image://theme/icon-m-website"' in content
    assert 'source: "image://theme/icon-m-document"' in content
    assert 'source: "image://theme/icon-m-developer-mode"' in content
    assert 'source: "image://theme/icon-m-mic"' in content

    # Subpage links
    assert 'pageStack.push(Qt.resolvedUrl("DisplaySettingsPage.qml"))' in content
    assert 'pageStack.push(Qt.resolvedUrl("WebSettingsPage.qml"))' in content
    assert 'pageStack.push(Qt.resolvedUrl("StorageSettingsPage.qml"))' in content
    assert 'pageStack.push(Qt.resolvedUrl("AssistantSettingsPage.qml"))' in content
    assert 'pageStack.push(Qt.resolvedUrl("SttSettingsPage.qml"))' in content


def test_main_page_pulley_menu_backup_and_settings():
    """Verify MainPage.qml pulley menu includes both Backup & Export and Settings."""
    path = Path("notesplus-sailfish/qml/pages/MainPage.qml")
    assert path.exists()
    content = path.read_text(encoding="utf-8")

    assert 'text: qsTr("Backup & Export")' in content
    assert 'pageStack.push(Qt.resolvedUrl("BackupPage.qml"))' in content
    assert 'text: qsTr("Settings")' in content
    assert 'pageStack.push(Qt.resolvedUrl("SettingsPage.qml"))' in content


def test_web_settings_page():
    """Verify WebSettingsPage.qml contains web server configuration without storage or backup."""
    path = Path("notesplus-sailfish/qml/pages/WebSettingsPage.qml")
    assert path.exists()
    content = path.read_text(encoding="utf-8")

    # Has web server options
    assert 'text: qsTr("Start Web Server on App Launch")' in content
    assert 'text: qsTr("Reject connections from public networks")' in content
    assert 'label: qsTr("Bind to Network Interface")' in content
    assert 'text: qsTr("Web Server Active")' in content
    assert 'label: qsTr("Session Expiry")' in content
    assert 'text: qsTr("SSL / TLS Certificate")' in content
    assert 'text: qsTr("Install Certificate")' in content

    # Does not contain storage or backup
    assert 'qsTr("Storage Location")' not in content
    assert 'notesPathField' not in content
    assert 'qsTr("Export All Notes as PDF")' not in content
    assert 'qsTr("Export All Notes as HTML5")' not in content


def test_storage_settings_page():
    """Verify StorageSettingsPage.qml contains storage location and search index maintenance without web or backup."""
    path = Path("notesplus-sailfish/qml/pages/StorageSettingsPage.qml")
    assert path.exists()
    content = path.read_text(encoding="utf-8")

    # Has storage and index options
    assert 'text: qsTr("Storage Location")' in content
    assert 'id: notesPathField' in content
    assert 'text: qsTr("Migrate Storage...")' in content
    assert 'MigrateStorageDialog.qml' in content
    assert 'text: qsTr("Reset to Default")' in content
    assert 'text: qsTr("Database & Search Index")' in content
    assert 'text: qsTr("Rebuild Search Index")' in content
    assert 'bridge.rebuild_index()' in content

    # Does not contain web or backup
    assert 'qsTr("Start Web Server on App Launch")' not in content
    assert 'qsTr("SSL / TLS Certificate")' not in content
    assert 'qsTr("Export All Notes as PDF")' not in content


def test_backup_page():
    """Verify BackupPage.qml contains full-database exports and does not contain settings."""
    path = Path("notesplus-sailfish/qml/pages/BackupPage.qml")
    assert path.exists()
    content = path.read_text(encoding="utf-8")

    assert 'title: qsTr("Backup & Export")' in content
    assert 'text: qsTr("Export All Notes as PDF")' in content
    assert 'text: qsTr("Export All Notes as HTML5")' in content
    assert 'bridge.export_all_pdf()' in content
    assert 'bridge.export_all_html()' in content
    assert 'bridge.any_pdf_export_exists()' in content

    assert 'notesPathField' not in content
    assert 'qsTr("Web Server")' not in content


def test_subpages_exist_and_instantiate_content():
    """Verify all category subpages exist and include their respective components."""
    disp_path = Path("notesplus-sailfish/qml/pages/DisplaySettingsPage.qml")
    assert disp_path.exists()
    assert "DisplaySettingsTab" in disp_path.read_text(encoding="utf-8")

    asst_path = Path("notesplus-sailfish/qml/pages/AssistantSettingsPage.qml")
    assert asst_path.exists()
    asst_content = asst_path.read_text(encoding="utf-8")
    assert "AssistantSettingsTab" in asst_content
    assert "AgentBridge" in asst_content

    stt_path = Path("notesplus-sailfish/qml/pages/SttSettingsPage.qml")
    assert stt_path.exists()
    stt_content = stt_path.read_text(encoding="utf-8")
    assert "SttSettingsTab" in stt_content
