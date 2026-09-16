import QtQuick 2.6
import Sailfish.Silica 1.0
import "../js/ThemeColors.js" as TC

Column {
    id: voiceInputBar
    width: parent.width
    spacing: Theme.paddingSmall

    property bool isSpeechRecording: false
    property bool isSpeechTranscribing: false
    property real liveAudioLevel: 0.0
    property var liveWaveform: [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
    property bool micEnabled: true
    property bool showSubmitButton: false
    property bool submitEnabled: false

    signal toggleMic()
    signal cancelRecording()
    signal submitClicked()

    // --- Recording / Transcribing Status Indicator ---
    Rectangle {
        width: parent.width - Theme.horizontalPageMargin * 2
        anchors.horizontalCenter: parent.horizontalCenter
        height: (voiceInputBar.isSpeechRecording || voiceInputBar.isSpeechTranscribing) ? Theme.itemSizeExtraSmall : 0
        color: Theme.rgba(Theme.highlightBackgroundColor, 0.15)
        radius: Theme.paddingSmall
        border.color: (voiceInputBar.isSpeechRecording && voiceInputBar.liveAudioLevel > 0.06) ?
                      Theme.rgba(Theme.highlightColor, 0.6) : Theme.rgba(Theme.primaryColor, 0.2)
        border.width: 1
        visible: voiceInputBar.isSpeechRecording || voiceInputBar.isSpeechTranscribing
        clip: true

        Behavior on height { NumberAnimation { duration: 150 } }
        Behavior on border.color { ColorAnimation { duration: 100 } }

        Row {
            anchors.centerIn: parent
            spacing: Theme.paddingMedium

            BusyIndicator {
                size: BusyIndicatorSize.ExtraSmall
                running: voiceInputBar.isSpeechTranscribing
                visible: voiceInputBar.isSpeechTranscribing
                anchors.verticalCenter: parent.verticalCenter
            }

            Row {
                spacing: 3
                anchors.verticalCenter: parent.verticalCenter
                visible: voiceInputBar.isSpeechRecording

                Repeater {
                    model: 7
                    Rectangle {
                        width: 3
                        readonly property real barVal: (voiceInputBar.liveWaveform && voiceInputBar.liveWaveform.length > index) ? voiceInputBar.liveWaveform[index] : 0.0
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
                    if (voiceInputBar.isSpeechTranscribing) {
                        return qsTr("Transcribing speech with offline Whisper...")
                    }
                    if (voiceInputBar.liveAudioLevel > 0.06) {
                        var percent = Math.round(voiceInputBar.liveAudioLevel * 100)
                        return qsTr("Hearing voice (%1%)... Tap mic to finish").arg(percent)
                    }
                    return qsTr("Listening... Speak into microphone")
                }
                color: Theme.primaryColor
                font.pixelSize: Theme.fontSizeSmall
                anchors.verticalCenter: parent.verticalCenter
            }

            Label {
                text: qsTr("Cancel")
                color: Theme.secondaryColor
                font.pixelSize: Theme.fontSizeExtraSmall
                visible: voiceInputBar.isSpeechRecording
                anchors.verticalCenter: parent.verticalCenter
                MouseArea {
                    anchors.fill: parent
                    anchors.margins: -Theme.paddingSmall
                    onClicked: voiceInputBar.cancelRecording()
                }
            }
        }
    }

    // --- Button Row (aligned with field right edge) ---
    Row {
        anchors.right: parent.right
        anchors.rightMargin: Theme.horizontalPageMargin
        layoutDirection: Qt.RightToLeft
        spacing: Theme.paddingSmall

        // Mic button with pulse halo
        Item {
            width: micBtn.width
            height: micBtn.height

            Rectangle {
                anchors.centerIn: parent
                width: parent.width + Theme.paddingSmall + Math.round(voiceInputBar.liveAudioLevel * 28)
                height: parent.height + Theme.paddingSmall + Math.round(voiceInputBar.liveAudioLevel * 28)
                radius: width / 2
                color: (voiceInputBar.liveAudioLevel > 0.06) ? TC.kVoiceActive : TC.kVoiceInactive
                opacity: voiceInputBar.isSpeechRecording ? Math.min(0.85, 0.25 + voiceInputBar.liveAudioLevel * 0.6) : 0.0
                visible: voiceInputBar.isSpeechRecording

                Behavior on width { NumberAnimation { duration: 60 } }
                Behavior on height { NumberAnimation { duration: 60 } }
                Behavior on opacity { NumberAnimation { duration: 60 } }
                Behavior on color { ColorAnimation { duration: 100 } }
            }

            IconButton {
                id: micBtn
                anchors.centerIn: parent
                icon.source: voiceInputBar.isSpeechRecording ? "image://theme/icon-m-clear" : "image://theme/icon-m-mic"
                highlighted: voiceInputBar.isSpeechRecording
                enabled: voiceInputBar.micEnabled
                onClicked: voiceInputBar.toggleMic()
            }
        }

        // Optional submit button (appears left of mic due to RTL)
        IconButton {
            visible: voiceInputBar.showSubmitButton
            icon.source: "image://theme/icon-m-send"
            enabled: voiceInputBar.submitEnabled
            onClicked: voiceInputBar.submitClicked()
        }
    }
}
