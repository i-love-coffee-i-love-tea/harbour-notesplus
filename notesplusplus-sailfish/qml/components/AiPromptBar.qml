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
    signal cancelOperation()

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
        visible: !promptBar.agentBusy && promptBar.sttEnabled
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

    // Submit button when STT is disabled and agent not busy
    IconButton {
        visible: !promptBar.agentBusy && !promptBar.sttEnabled
        anchors.right: parent.right
        anchors.rightMargin: Theme.horizontalPageMargin
        icon.source: "image://theme/icon-m-send"
        enabled: promptField.text.trim().length > 0
        onClicked: {
            if (promptField.text.trim().length > 0) {
                promptBar.submitPrompt(promptField.text)
            }
        }
    }

    // Cancel button when agent is busy
    BackgroundItem {
        visible: promptBar.agentBusy
        width: parent.width - Theme.horizontalPageMargin * 2
        anchors.horizontalCenter: parent.horizontalCenter
        height: Theme.itemSizeSmall

        Rectangle {
            anchors.fill: parent
            radius: Theme.paddingSmall
            color: parent.pressed ? Theme.rgba(Theme.highlightColor, 0.3) : Theme.rgba(Theme.highlightColor, 0.15)
            border.color: Theme.rgba(Theme.highlightColor, 0.4)
            border.width: 1
        }

        Row {
            anchors.centerIn: parent
            spacing: Theme.paddingMedium

            Icon {
                source: "image://theme/icon-m-close"
                width: Theme.iconSizeSmall
                height: Theme.iconSizeSmall
                color: Theme.highlightColor
                anchors.verticalCenter: parent.verticalCenter
            }

            Label {
                text: qsTr("Cancel")
                color: Theme.highlightColor
                font.pixelSize: Theme.fontSizeSmall
                anchors.verticalCenter: parent.verticalCenter
            }
        }

        onClicked: promptBar.cancelOperation()
    }
}
