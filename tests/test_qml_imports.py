"""Automated regression tests to verify QML component imports and resolution."""
import os
import re
from pathlib import Path


def get_all_qml_files():
    qml_dir = Path("notesplus-sailfish/qml")
    return list(qml_dir.rglob("*.qml"))


def test_qml_component_imports_resolvable():
    """Verify that every custom QML component referenced in any QML file is properly imported."""
    qml_dir = Path("notesplus-sailfish/qml")
    assert qml_dir.exists(), "QML directory must exist"

    all_components = {}
    for p in qml_dir.rglob("*.qml"):
        all_components[p.stem] = p

    unresolved_usages = []

    for p in qml_dir.rglob("*.qml"):
        text = p.read_text(encoding="utf-8")
        imports = []
        for line in text.splitlines():
            line = line.strip()
            if line.startswith("import "):
                m = re.match(r"import\s+[\"\x27]([^\x27\"]+)[\"\x27]", line)
                if m:
                    imports.append(m.group(1))

        available_dirs = {p.parent.resolve()}
        for imp in imports:
            resolved = (p.parent / imp).resolve()
            available_dirs.add(resolved)

        usages = set(re.findall(r"\b([A-Z][a-zA-Z0-9_]+)\s*\{", text))
        for u in usages:
            if u in all_components:
                def_path = all_components[u]
                if def_path.parent.resolve() not in available_dirs:
                    unresolved_usages.append((
                        str(p.relative_to(qml_dir)),
                        u,
                        str(def_path.relative_to(qml_dir))
                    ))

    assert not unresolved_usages, f"Unresolved QML component references found: {unresolved_usages}"


def test_stt_settings_tab_imports_stt_model_delegate():
    path = Path("notesplus-sailfish/qml/components/settings/SttSettingsTab.qml")
    content = path.read_text(encoding="utf-8")
    assert 'import "../ai"' in content, "SttSettingsTab.qml must import ../ai to resolve SttModelDelegate"


def test_model_download_dialog_imports_stt_model_delegate():
    path = Path("notesplus-sailfish/qml/dialogs/ModelDownloadDialog.qml")
    content = path.read_text(encoding="utf-8")
    assert 'import "../components/ai"' in content, "ModelDownloadDialog.qml must import ../components/ai to resolve SttModelDelegate"
