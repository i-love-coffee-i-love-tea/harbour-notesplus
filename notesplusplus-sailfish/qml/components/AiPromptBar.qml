import QtQuick 2.6
import Sailfish.Silica 1.0

Column {
    id: promptBar
    width: parent.width
    spacing: Theme.paddingSmall

    property alias text: promptField.text
    property alias placeholderText: promptField.placeholderText
    property bool agentBusy: false
    property bool isSpeechRecording: false
    property bool isSpeechTranscribing: false
    property real liveAudioLevel: 0.0
    property var liveWaveform: [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
    property bool sttEnabled: true

    signal submitPrompt(string text)
    signal toggleMic()
    signal cancelRecording()

    // Input Area
    TextArea {
        id: promptField
        width: parent.width - Theme.horizontalPageMargin * 2
        anchors.horizontalCenter: parent.horizontalCenter
        placeholderText: qsTr("Ask anything about your notes or request changes...")
        label: qsTr("Prompt")
        height: Math.max(Theme.itemSizeMedium, implicitHeight)
    }

    // Voice Input with Submit
    VoiceInputBar {
        visible: promptBar.sttEnabled
        isSpeechRecording: promptBar.isSpeechRecording
        isSpeechTranscribing: promptBar.isSpeechTranscribing
        liveAudioLevel: promptBar.liveAudioLevel
        liveWaveform: promptBar.liveWaveform
        micEnabled: !promptBar.agentBusy && !promptBar.isSpeechTranscribing
        showSubmitButton: true
        submitEnabled: !promptBar.agentBusy && !promptBar.isSpeechTranscribing && promptField.text.trim().length > 0
        onToggleMic: promptBar.toggleMic()
        onCancelRecording: promptBar.cancelRecording()
        onSubmitClicked: {
            if (promptField.text.trim().length > 0) {
                promptBar.submitPrompt(promptField.text)
            }
        }
    }
}
