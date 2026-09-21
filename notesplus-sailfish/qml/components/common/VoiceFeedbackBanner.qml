import QtQuick 2.6
import Sailfish.Silica 1.0

Rectangle {
    id: feedbackBanner
    width: parent ? Math.min(parent.width - Theme.horizontalPageMargin * 2, Theme.itemSizeHuge * 3) : Screen.width - Theme.horizontalPageMargin * 2
    height: active ? (Theme.itemSizeExtraSmall + Theme.paddingSmall) : 0
    anchors.horizontalCenter: parent ? parent.horizontalCenter : undefined
    anchors.bottom: parent ? parent.bottom : undefined
    anchors.bottomMargin: (Qt.inputMethod.visible ? Math.min(Qt.inputMethod.keyboardRectangle.height, Screen.height * 0.5) : 0) + Theme.paddingLarge
    z: 60

    property var controller: null
    readonly property bool isSpeechRecording: controller ? controller.isRecording : (typeof speechBridge !== "undefined" && speechBridge && speechBridge.is_recording)
    readonly property bool isSpeechTranscribing: controller ? controller.isTranscribing : (typeof speechBridge !== "undefined" && speechBridge && speechBridge.is_transcribing)
    readonly property real liveAudioLevel: controller ? controller.audioLevel : ((typeof speechBridge !== "undefined" && speechBridge && isSpeechRecording) ? speechBridge.audio_level : 0.0)
    readonly property var liveWaveform: (controller && controller.liveWaveform) ? controller.liveWaveform : [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]

    readonly property bool active: isSpeechRecording || isSpeechTranscribing

    opacity: active ? 1.0 : 0.0
    visible: opacity > 0.001
    clip: true

    radius: Theme.paddingMedium
    color: Theme.rgba(Theme.overlayBackgroundColor, 0.95)
    border.color: (isSpeechRecording && liveAudioLevel > 0.06) ?
                  Theme.rgba(Theme.highlightColor, 0.7) : Theme.rgba(Theme.primaryColor, 0.25)
    border.width: 1

    Behavior on opacity { FadeAnimation { duration: 180 } }
    Behavior on height { NumberAnimation { duration: 150 } }
    Behavior on border.color { ColorAnimation { duration: 100 } }

    signal cancelRequested()

    function cancel() {
        if (controller && typeof controller.cancelRecording === "function") {
            controller.cancelRecording()
        } else if (typeof speechBridge !== "undefined" && speechBridge && speechBridge.is_recording) {
            speechBridge.cancel_recording()
        }
        cancelRequested()
    }

    Row {
        anchors.fill: parent
        anchors.leftMargin: Theme.paddingMedium
        anchors.rightMargin: Theme.paddingMedium
        spacing: Theme.paddingSmall

        BusyIndicator {
            size: BusyIndicatorSize.ExtraSmall
            running: feedbackBanner.isSpeechTranscribing
            visible: feedbackBanner.isSpeechTranscribing
            anchors.verticalCenter: parent.verticalCenter
        }

        // 7-bar waveform visualization
        Row {
            spacing: 3
            anchors.verticalCenter: parent.verticalCenter
            visible: feedbackBanner.isSpeechRecording

            Repeater {
                model: 7
                Rectangle {
                    width: 3
                    readonly property real barVal: (feedbackBanner.liveWaveform && feedbackBanner.liveWaveform.length > index) ? feedbackBanner.liveWaveform[index] : 0.0
                    height: Math.max(4, Math.min(Theme.itemSizeExtraSmall - Theme.paddingSmall, Math.round(barVal * (Theme.itemSizeExtraSmall - Theme.paddingSmall))))
                    radius: 1.5
                    color: (barVal > 0.06) ? Theme.highlightColor : Theme.secondaryColor
                    anchors.verticalCenter: parent.verticalCenter

                    Behavior on height { NumberAnimation { duration: 50 } }
                    Behavior on color { ColorAnimation { duration: 80 } }
                }
            }
        }

        // Audio level indicator dot
        Rectangle {
            width: Theme.paddingSmall + Math.round(feedbackBanner.liveAudioLevel * 10)
            height: width
            radius: width / 2
            anchors.verticalCenter: parent.verticalCenter
            visible: feedbackBanner.isSpeechRecording
            color: (feedbackBanner.liveAudioLevel > 0.06) ? Theme.highlightColor : Theme.secondaryColor
            opacity: 0.8
            Behavior on width { NumberAnimation { duration: 60 } }
        }

        Label {
            text: {
                if (feedbackBanner.isSpeechTranscribing) {
                    return qsTr("Transcribing speech with offline Whisper...")
                }
                if (feedbackBanner.liveAudioLevel > 0.06) {
                    var percent = Math.round(feedbackBanner.liveAudioLevel * 100)
                    return qsTr("Hearing voice (%1%)... Tap mic to finish").arg(percent)
                }
                return qsTr("Listening... Speak into microphone")
            }
            color: Theme.primaryColor
            font.pixelSize: Theme.fontSizeSmall
            anchors.verticalCenter: parent.verticalCenter
            truncationMode: TruncationMode.Fade
            width: parent.width - (cancelItem.visible ? cancelItem.width : 0) - (feedbackBanner.isSpeechTranscribing ? Theme.itemSizeExtraSmall : (feedbackBanner.isSpeechRecording ? (7 * 6 + Theme.paddingSmall + 10) : 0)) - Theme.paddingMedium * 2
        }

        Item {
            id: cancelItem
            width: cancelLabel.implicitWidth + Theme.paddingSmall * 2
            height: parent.height
            anchors.verticalCenter: parent.verticalCenter
            visible: feedbackBanner.isSpeechRecording

            Label {
                id: cancelLabel
                anchors.centerIn: parent
                text: qsTr("Cancel")
                color: Theme.secondaryColor
                font.pixelSize: Theme.fontSizeExtraSmall
            }

            MouseArea {
                anchors.fill: parent
                onClicked: feedbackBanner.cancel()
            }
        }
    }
}
