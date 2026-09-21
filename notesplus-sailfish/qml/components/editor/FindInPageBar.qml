import QtQuick 2.6
import Sailfish.Silica 1.0

Rectangle {
    id: findBar

    property alias searchTerm: searchField.text
    property alias placeholderText: searchField.placeholderText
    property alias searchFieldComponent: searchField
    property int currentMatchIndex: -1
    property int totalMatches: 0

    signal nextClicked()
    signal previousClicked()
    signal closeClicked()
    signal textChanged(string text)
    signal searchSubmitted(string text)
    signal voiceInputRequested(var target)

    function focusSearchField() {
        searchField.forceActiveFocus()
    }

    function clear() {
        searchField.text = ""
    }

    height: Theme.itemSizeMedium
    color: Theme.rgba(Theme.overlayBackgroundColor, 0.95)
    border.color: Theme.rgba(Theme.highlightColor, 0.4)
    border.width: 1
    radius: Theme.paddingSmall
    clip: true

    Row {
        anchors.fill: parent
        anchors.leftMargin: Theme.paddingSmall
        anchors.rightMargin: Theme.paddingSmall
        spacing: Theme.paddingSmall

        SearchField {
            id: searchField
            width: parent.width - buttonRow.width - Theme.paddingSmall * 2
            anchors.verticalCenter: parent.verticalCenter
            placeholderText: qsTr("Find in page...")
            EnterKey.enabled: true
            EnterKey.iconSource: "image://theme/icon-m-enter-next"
            EnterKey.onClicked: {
                findBar.searchSubmitted(searchField.text)
                findBar.nextClicked()
            }
            onTextChanged: {
                findBar.textChanged(text)
            }
        }

        Row {
            id: buttonRow
            anchors.verticalCenter: parent.verticalCenter
            spacing: Theme.paddingSmall / 2

            Label {
                id: counterLabel
                anchors.verticalCenter: parent.verticalCenter
                text: {
                    if (!searchField.text || searchField.text.trim().length === 0) return ""
                    if (findBar.totalMatches === 0) return "0 / 0"
                    var currentNumber = (findBar.currentMatchIndex >= 0 ? findBar.currentMatchIndex : 0) + 1
                    return currentNumber + " / " + findBar.totalMatches
                }
                color: findBar.totalMatches > 0 ? Theme.highlightColor : Theme.secondaryColor
                font.pixelSize: Theme.fontSizeExtraSmall
                visible: text.length > 0
            }

            IconButton {
                id: prevBtn
                icon.source: "image://theme/icon-m-up"
                anchors.verticalCenter: parent.verticalCenter
                enabled: findBar.totalMatches > 0
                opacity: enabled ? 1.0 : 0.3
                onClicked: findBar.previousClicked()
            }

            IconButton {
                id: nextBtn
                icon.source: "image://theme/icon-m-down"
                anchors.verticalCenter: parent.verticalCenter
                enabled: findBar.totalMatches > 0
                opacity: enabled ? 1.0 : 0.3
                onClicked: findBar.nextClicked()
            }

            Item {
                id: micItem
                width: visible ? Theme.itemSizeExtraSmall : 0
                height: Theme.itemSizeExtraSmall
                anchors.verticalCenter: parent.verticalCenter
                visible: typeof app === "undefined" || !app || app.sttEnabled !== false

                readonly property bool isRecording: typeof speechBridge !== "undefined" && speechBridge && speechBridge.is_recording
                readonly property real liveAudioLevel: (isRecording && typeof speechBridge !== "undefined" && speechBridge) ? speechBridge.audio_level : 0.0

                Rectangle {
                    anchors.centerIn: parent
                    width: Math.min(parent.width, Theme.iconSizeSmall + Theme.paddingSmall + Math.round(micItem.liveAudioLevel * 20))
                    height: width
                    radius: width / 2
                    color: (micItem.liveAudioLevel > 0.06) ? Theme.highlightColor : Theme.secondaryColor
                    opacity: micItem.isRecording ? Math.min(0.85, 0.25 + micItem.liveAudioLevel * 0.6) : 0.0
                    visible: micItem.isRecording

                    Behavior on width { NumberAnimation { duration: 60 } }
                    Behavior on height { NumberAnimation { duration: 60 } }
                    Behavior on opacity { NumberAnimation { duration: 60 } }
                }

                IconButton {
                    id: micBtn
                    anchors.centerIn: parent
                    icon.source: micItem.isRecording ? "image://theme/icon-m-clear" : "image://theme/icon-m-mic"
                    highlighted: micItem.isRecording
                    onClicked: {
                        findBar.voiceInputRequested(searchField)
                    }
                }
            }

            IconButton {
                id: closeBtn
                icon.source: "image://theme/icon-m-close"
                anchors.verticalCenter: parent.verticalCenter
                onClicked: findBar.closeClicked()
            }
        }
    }
}
