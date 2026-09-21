"""Automated integration tests for offline speech-to-text voice input across editors and search."""
import re
from pathlib import Path


def test_voice_input_controller_structure():
    """Verify VoiceInputController.qml exposes required properties, signals, checks, and methods."""
    path = Path("notesplus-sailfish/qml/components/common/VoiceInputController.qml")
    assert path.exists(), "VoiceInputController.qml must exist in qml/components/common/"
    content = path.read_text(encoding="utf-8")

    # Property definitions
    assert "property var activeTarget: null" in content
    assert "readonly property bool isRecording" in content
    assert "readonly property bool isTranscribing" in content
    assert "readonly property real audioLevel" in content
    assert "readonly property var liveWaveform" in content
    assert "readonly property bool hasInstalledModels" in content

    # Signal definitions
    assert "signal recordingStarted()" in content
    assert "signal recordingStopped()" in content
    assert "signal recordingCanceled()" in content
    assert "signal textInserted(string text)" in content
    assert "signal errorOccurred(string message)" in content

    # Method definitions
    assert "function toggleMic(targetComponent)" in content
    assert "function cancelRecording()" in content
    assert "function insertTextAtCursor(targetComponent, text)" in content

    # Model readiness check & routing
    assert "speechBridge.has_installed_models" in content
    assert "ModelDownloadDialog.qml" in content
    assert "app.sttEnabled" in content

    # SpeechBridge connections
    assert "target: (typeof speechBridge !== \"undefined\" && speechBridge) ? speechBridge : null" in content
    assert "onTranscription_completed:" in content
    assert "onError_occurred:" in content


def test_voice_feedback_banner_structure():
    """Verify VoiceFeedbackBanner.qml displays 7-bar waveform, level indicator, status label, and cancel action."""
    path = Path("notesplus-sailfish/qml/components/common/VoiceFeedbackBanner.qml")
    assert path.exists(), "VoiceFeedbackBanner.qml must exist in qml/components/common/"
    content = path.read_text(encoding="utf-8")

    # Property bindings
    assert "property var controller: null" in content
    assert "readonly property bool isSpeechRecording" in content
    assert "readonly property bool isSpeechTranscribing" in content
    assert "readonly property real liveAudioLevel" in content
    assert "readonly property var liveWaveform" in content
    assert "readonly property bool active:" in content

    # Visual indicators
    assert "BusyIndicator {" in content
    assert "model: 7" in content  # 7-bar waveform repeater
    assert "Behavior on opacity" in content
    assert "Behavior on height" in content

    # Status labels & actions
    assert 'qsTr("Transcribing speech with offline Whisper...")' in content
    assert 'qsTr("Hearing voice (%1%)... Tap mic to finish")' in content
    assert 'qsTr("Listening... Speak into microphone")' in content
    assert 'qsTr("Cancel")' in content
    assert "signal cancelRequested()" in content
    assert "function cancel()" in content


def test_in_place_edit_sidebar_voice_integration():
    """Verify InPlaceEditSidebar.qml includes showVoiceInput, voiceInputRequested signal, and mic button."""
    path = Path("notesplus-sailfish/qml/components/editor/InPlaceEditSidebar.qml")
    assert path.exists()
    content = path.read_text(encoding="utf-8")

    assert "property bool showVoiceInput: true" in content
    assert "signal voiceInputRequested()" in content
    assert "id: micItem" in content
    assert "id: micBtn" in content
    assert "inPlaceSidebar.showVoiceInput" in content
    assert "inPlaceSidebar.voiceInputRequested()" in content


def test_journal_voice_input_exclusion():
    """Verify InPlaceEditSidebar explicitly sets showVoiceInput: false in the journal section of MainPage."""
    path = Path("notesplus-sailfish/qml/pages/MainPage.qml")
    assert path.exists()
    content = path.read_text(encoding="utf-8")

    # InPlaceEditSidebar in MainPage must explicitly disable voice input
    assert "InPlaceEditSidebar {" in content
    assert "showVoiceInput: false" in content


def test_page_view_voice_wiring():
    """Verify PageView.qml instantiates VoiceInputController and VoiceFeedbackBanner and wires note block editing."""
    path = Path("notesplus-sailfish/qml/pages/PageView.qml")
    assert path.exists()
    content = path.read_text(encoding="utf-8")

    assert "VoiceInputController {" in content
    assert "VoiceFeedbackBanner {" in content
    assert "voiceController.cancelRecording()" in content
    assert "onVoiceInputRequested:" in content
    assert "voiceController.toggleMic(target)" in content


def test_editor_toolbar_and_page_source_editor_voice_integration():
    """Verify EditorToolbar.qml and PageSourceEditor.qml include voice dictation action and controller."""
    toolbar_path = Path("notesplus-sailfish/qml/components/editor/EditorToolbar.qml")
    assert toolbar_path.exists()
    toolbar_content = toolbar_path.read_text(encoding="utf-8")

    assert 'itemId: "voice"' in toolbar_content
    assert 'actionType: "voice"' in toolbar_content
    assert "signal voiceInputRequested(" in toolbar_content
    assert "editorToolbar.voiceInputRequested(targetTextArea)" in toolbar_content

    editor_path = Path("notesplus-sailfish/qml/dialogs/PageSourceEditor.qml")
    assert editor_path.exists()
    editor_content = editor_path.read_text(encoding="utf-8")

    assert "VoiceInputController {" in editor_content
    assert "VoiceFeedbackBanner {" in editor_content
    assert "onVoiceInputRequested:" in editor_content
    assert "voiceController.toggleMic(target || textArea)" in editor_content
    assert "voiceController.cancelRecording()" in editor_content


def test_main_page_search_voice_wiring():
    """Verify MainPage.qml has a microphone trigger button beside the searchField wired to VoiceInputController."""
    path = Path("notesplus-sailfish/qml/pages/MainPage.qml")
    assert path.exists()
    content = path.read_text(encoding="utf-8")

    assert "id: searchMicItem" in content
    assert "id: searchMicBtn" in content
    assert "voiceController.toggleMic(searchField)" in content
    assert "VoiceInputController {" in content
    assert "VoiceFeedbackBanner {" in content


def test_find_in_page_bar_voice_wiring():
    """Verify FindInPageBar.qml includes a microphone action button emitting voiceInputRequested."""
    path = Path("notesplus-sailfish/qml/components/editor/FindInPageBar.qml")
    assert path.exists()
    content = path.read_text(encoding="utf-8")

    assert "signal voiceInputRequested(var target)" in content
    assert "id: micItem" in content
    assert "id: micBtn" in content
    assert "findBar.voiceInputRequested(searchField)" in content


def test_translations_pro_includes_voice_components():
    """Verify translations.pro indexes components/common and components/editor."""
    path = Path("notesplus-sailfish/translations/translations.pro")
    assert path.exists()
    content = path.read_text(encoding="utf-8")

    assert "qml/components/common/*.qml" in content
    assert "qml/components/editor/*.qml" in content


def simulate_cursor_insertion(full_text, selection_start, selection_end, cursor_position, transcribed_text):
    """Python simulation mirroring VoiceInputController.insertTextAtCursor logic."""
    if not transcribed_text or not transcribed_text.strip():
        return full_text, cursor_position

    insert_str = transcribed_text.strip()
    full_text = full_text or ""

    if selection_start is not None and selection_end is not None and selection_start != selection_end:
        start = max(0, min(selection_start, selection_end))
        end = min(len(full_text), max(selection_start, selection_end))
    elif cursor_position is not None and cursor_position >= 0:
        start = min(len(full_text), cursor_position)
        end = start
    else:
        start = len(full_text)
        end = len(full_text)

    pre_text = full_text[:start]
    post_text = full_text[end:]

    # Smart spacing before
    if len(pre_text) > 0 and not pre_text[-1].isspace():
        insert_str = " " + insert_str

    # Smart spacing after
    if len(post_text) > 0 and not post_text[0].isspace():
        insert_str = insert_str + " "

    new_text = pre_text + insert_str + post_text
    new_cursor_pos = len(pre_text) + len(insert_str)

    return new_text, new_cursor_pos


def test_cursor_insertion_semantics():
    """Verify smart whitespace padding, cursor positioning, and selection replacement."""
    # 1. Insertion into empty field
    text, pos = simulate_cursor_insertion("", 0, 0, 0, "hello world")
    assert text == "hello world"
    assert pos == 11

    # 2. Append at end of existing text without trailing space
    text, pos = simulate_cursor_insertion("Existing note", None, None, 13, "more text")
    assert text == "Existing note more text"
    assert pos == len("Existing note more text")

    # 3. Append when existing text already has trailing space
    text, pos = simulate_cursor_insertion("Existing note ", None, None, 14, "more text")
    assert text == "Existing note more text"
    assert pos == len("Existing note more text")

    # 4. Insert in the middle of a sentence
    text, pos = simulate_cursor_insertion("Hello world", None, None, 5, "brave new")
    assert text == "Hello brave new world"
    assert pos == len("Hello brave new")

    # 5. Replace selected text
    text, pos = simulate_cursor_insertion("The bad fox jumps", 4, 7, 7, "quick brown")
    assert text == "The quick brown fox jumps"
    assert pos == len("The quick brown")

    # 6. Replace entire selection at beginning
    text, pos = simulate_cursor_insertion("Initial start", 0, 7, 7, "Fresh")
    assert text == "Fresh start"
    assert pos == 5

    # 7. Blank / whitespace-only speech transcription does nothing
    text, pos = simulate_cursor_insertion("Preserved text", None, None, 5, "   ")
    assert text == "Preserved text"
    assert pos == 5
