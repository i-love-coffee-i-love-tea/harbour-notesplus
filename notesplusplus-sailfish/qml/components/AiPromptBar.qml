import QtQuick 2.0
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

    // Recording / Transcribing Status Indicator
    Rectangle {
        width: parent.width - Theme.horizontalPageMargin * 2
        anchors.horizontalCenter: parent.horizontalCenter
        height: (promptBar.isSpeechRecording || promptBar.isSpeechTranscribing) ? Theme.itemSizeExtraSmall : 0
        color: Theme.rgba(Theme.highlightBackgroundColor, 0.15)
        radius: Theme.paddingSmall
        border.color: (promptBar.isSpeechRecording && promptBar.liveAudioLevel > 0.06) ?
                      Theme.rgba(Theme.highlightColor, 0.6) : Theme.rgba(Theme.highlightColor, 0.3)
        border.width: 1
        visible: promptBar.isSpeechRecording || promptBar.isSpeechTranscribing
        clip: true

        Behavior on height { NumberAnimation { duration: 150 } }
        Behavior on border.color { ColorAnimation { duration: 100 } }

        Row {
            anchors.centerIn: parent
            spacing: Theme.paddingMedium

            BusyIndicator {
                size: BusyIndicatorSize.ExtraSmall
                running: promptBar.isSpeechTranscribing
                visible: promptBar.isSpeechTranscribing
                anchors.verticalCenter: parent.verticalCenter
            }

            // Live Multi-Bar Waveform Visualizer
            Row {
                spacing: 3
                anchors.verticalCenter: parent.verticalCenter
                visible: promptBar.isSpeechRecording

                Repeater {
                    model: 7
                    Rectangle {
                        id: waveBar
                        width: 3
                        readonly property real barVal: (promptBar.liveWaveform && promptBar.liveWaveform.length > index) ? promptBar.liveWaveform[index] : 0.0
                        height: Math.max(4, Math.min(Theme.itemSizeExtraSmall - Theme.paddingMedium, Math.round(barVal * (Theme.itemSizeExtraSmall - Theme.paddingMedium))))
                        radius: 1.5
                        color: (barVal > 0.06) ? Theme.highlightColor : Theme.secondaryColor
                        anchors.verticalCenter: parent.verticalCenter

                        Behavior on height { NumberAnimation { duration: 50 } }
                        Behavior on color { ColorAnimation { duration: 80 } }
                    }
                }
            }

            Label {
                text: {
                    if (promptBar.isSpeechTranscribing) {
                        return qsTr("Transcribing speech with offline Whisper...")
                    }
                    if (promptBar.liveAudioLevel > 0.06) {
                        var percent = Math.round(promptBar.liveAudioLevel * 100)
                        return qsTr("Hearing voice (%1%)... Tap mic to finish").arg(percent)
                    }
                    return qsTr("Listening... Speak into microphone")
                }
                color: (promptBar.isSpeechRecording && promptBar.liveAudioLevel > 0.06) ?
                       Theme.primaryColor : Theme.highlightColor
                font.pixelSize: Theme.fontSizeSmall
                anchors.verticalCenter: parent.verticalCenter
            }

            Label {
                text: qsTr("Cancel")
                color: Theme.secondaryHighlightColor
                font.pixelSize: Theme.fontSizeExtraSmall
                visible: promptBar.isSpeechRecording
                anchors.verticalCenter: parent.verticalCenter
                MouseArea {
                    anchors.fill: parent
                    anchors.margins: -Theme.paddingSmall
                    onClicked: promptBar.cancelRecording()
                }
            }
        }
    }

    // Input Area
    Column {
        width: parent.width - Theme.horizontalPageMargin * 2
        anchors.horizontalCenter: parent.horizontalCenter
        spacing: Theme.paddingSmall

        TextArea {
            id: promptField
            width: parent.width
            placeholderText: qsTr("Ask anything about your notes or request changes...")
            label: qsTr("Prompt")
            height: Math.max(Theme.itemSizeMedium, implicitHeight)
        }

        Row {
            anchors.right: parent.right
            layoutDirection: Qt.RightToLeft
            spacing: Theme.paddingSmall

            IconButton {
                id: sendBtn
                icon.source: "image://theme/icon-m-send"
                enabled: !promptBar.agentBusy && !promptBar.isSpeechTranscribing && promptField.text.trim().length > 0
                onClicked: {
                    if (promptField.text.trim().length > 0) {
                        promptBar.submitPrompt(promptField.text)
                    }
                }
            }

            Item {
                id: micBtnContainer
                width: micBtn.width
                height: micBtn.height
                visible: promptBar.sttEnabled

                // Dynamic audio volume halo reacting directly to live microphone voice input
                Rectangle {
                    id: micPulseHalo
                    anchors.centerIn: parent
                    width: parent.width + Theme.paddingSmall + Math.round(promptBar.liveAudioLevel * 28)
                    height: parent.height + Theme.paddingSmall + Math.round(promptBar.liveAudioLevel * 28)
                    radius: width / 2
                    color: (promptBar.liveAudioLevel > 0.06) ? "#44ff88" : "#ff4444"
                    opacity: promptBar.isSpeechRecording ? Math.min(0.85, 0.25 + promptBar.liveAudioLevel * 0.6) : 0.0
                    visible: promptBar.isSpeechRecording

                    Behavior on width { NumberAnimation { duration: 60 } }
                    Behavior on height { NumberAnimation { duration: 60 } }
                    Behavior on opacity { NumberAnimation { duration: 60 } }
                    Behavior on color { ColorAnimation { duration: 100 } }
                }

                IconButton {
                    id: micBtn
                    anchors.centerIn: parent
                    icon.source: promptBar.isSpeechRecording ? "image://theme/icon-m-clear" : "image://theme/icon-m-mic"
                    highlighted: promptBar.isSpeechRecording
                    enabled: !promptBar.agentBusy && !promptBar.isSpeechTranscribing
                    onClicked: promptBar.toggleMic()
                }
            }
        }
    }
}
