import QtQuick 2.6
import Sailfish.Silica 1.0

Item {
    id: voiceInputController

    property var speechBridgeRef: null
    property var activeTarget: null
    property var transcriptionHandler: null
    property var textFilter: null
    readonly property bool isRecording: speechBridgeRef && speechBridgeRef.is_recording
    readonly property bool isTranscribing: speechBridgeRef && speechBridgeRef.is_transcribing
    readonly property real audioLevel: (speechBridgeRef && isRecording) ? speechBridgeRef.audio_level : 0.0
    readonly property var liveWaveform: {
        if (speechBridgeRef && speechBridgeRef.waveform_json && isRecording) {
            try {
                return JSON.parse(speechBridgeRef.waveform_json) || [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
            } catch (e) {
                return [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
            }
        }
        return [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
    }
    readonly property bool hasInstalledModels: speechBridgeRef && speechBridgeRef.has_installed_models

    signal recordingStarted()
    signal recordingStopped()
    signal recordingCanceled()
    signal textInserted(string text)
    signal errorOccurred(string message)

    onSpeechBridgeRefChanged: {
        if (speechBridgeRef) {
            console.log("[VoiceInputController] speechBridgeRef set, connecting signals programmatically")
            speechBridgeRef.transcription_completed.connect(handleTranscription)
            speechBridgeRef.error_occurred.connect(handleError)
        }
    }

    function handleTranscription(text) {
        var filtered = textFilter ? textFilter(text) : text
        if (transcriptionHandler) {
            transcriptionHandler(filtered)
        } else if (activeTarget) {
            insertTextAtCursor(activeTarget, filtered)
            activeTarget = null
        }
    }

    function handleError(message) {
        var errMsg = (typeof message !== "undefined" && message) ? message : ""
        if (errMsg.length > 0) {
            if (typeof notification !== "undefined" && notification) {
                notification.show(errMsg)
            }
            errorOccurred(errMsg)
        }
    }

    function toggleMic(targetComponent) {
        if (targetComponent) {
            activeTarget = targetComponent
        }

        if (typeof app !== "undefined" && app && app.sttEnabled === false) {
            if (typeof notification !== "undefined" && notification) {
                notification.show(qsTr("Voice input is disabled in Settings"))
            }
            return
        }

        if (!speechBridgeRef) {
            if (typeof notification !== "undefined" && notification) {
                notification.show(qsTr("Speech recognition is unavailable."))
            }
            return
        }

        if (speechBridgeRef.is_recording) {
            speechBridgeRef.stop_recording_and_transcribe()
            recordingStopped()
            return
        }

        if (speechBridgeRef.is_transcribing) {
            return
        }

        if (!speechBridgeRef.has_installed_models) {
            if (typeof notification !== "undefined" && notification) {
                notification.show(qsTr("No speech model installed. Please download a model."))
            }
            if (typeof pageStack !== "undefined" && pageStack) {
                pageStack.push(Qt.resolvedUrl("../../dialogs/ModelDownloadDialog.qml"))
            }
            return
        }

        if (speechBridgeRef.start_recording()) {
            recordingStarted()
        }
    }

    function cancelRecording() {
        if (speechBridgeRef && speechBridgeRef.is_recording) {
            speechBridgeRef.cancel_recording()
            recordingCanceled()
        }
        activeTarget = null
    }

    function insertTextAtCursor(targetComponent, text) {
        var target = targetComponent || activeTarget
        if (!target) return
        if (typeof text !== "string" || text.trim().length === 0) return

        var insertStr = text.trim()

        // Focus first so the input context reattaches before text is set
        if (typeof target.forceActiveFocus === "function") {
            target.forceActiveFocus()
        }

        var existing = (typeof target.text === "string") ? target.text : ""
        if (existing.length > 0) {
            target.text = existing + " " + insertStr
        } else {
            target.text = insertStr
        }
        textInserted(target.text)
    }
}
