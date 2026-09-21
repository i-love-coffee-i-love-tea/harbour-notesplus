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

    height: searchField.height + navRow.height
    color: Theme.highlightDimmerColor
    opacity: 0.85

    Column {
        anchors.fill: parent
        spacing: 0

        Row {
            width: parent.width
            spacing: 0

            SearchField {
                id: searchField
                width: parent.width - micBtnRow.width - clearBtn.width
                anchors.verticalCenter: parent.verticalCenter
                placeholderText: qsTr("Find in page...")
                font.pixelSize: Theme.fontSizeSmall
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

            Item {
                id: micBtnRow
                width: visible ? Theme.itemSizeMedium : 0
                height: Theme.itemSizeMedium
                anchors.verticalCenter: parent.verticalCenter
                visible: typeof app === "undefined" || !app || app.sttEnabled !== false

                readonly property bool isRecording: typeof speechBridge !== "undefined" && speechBridge && speechBridge.is_recording
                readonly property real liveAudioLevel: (isRecording && typeof speechBridge !== "undefined" && speechBridge) ? speechBridge.audio_level : 0.0

                Rectangle {
                    anchors.centerIn: parent
                    width: Math.min(parent.width, Theme.iconSizeMedium + Theme.paddingSmall + Math.round(micBtnRow.liveAudioLevel * 20))
                    height: width
                    radius: width / 2
                    color: (micBtnRow.liveAudioLevel > 0.06) ? Theme.highlightColor : Theme.secondaryColor
                    opacity: micBtnRow.isRecording ? Math.min(0.85, 0.25 + micBtnRow.liveAudioLevel * 0.6) : 0.0
                    visible: micBtnRow.isRecording

                    Behavior on width { NumberAnimation { duration: 60 } }
                    Behavior on height { NumberAnimation { duration: 60 } }
                    Behavior on opacity { NumberAnimation { duration: 60 } }
                }

                IconButton {
                    anchors.centerIn: parent
                    icon.source: micBtnRow.isRecording ? "image://theme/icon-m-clear" : "image://theme/icon-m-mic"
                    highlighted: micBtnRow.isRecording
                    onClicked: {
                        findBar.voiceInputRequested(searchField)
                    }
                }
            }

            Item {
                id: clearBtn
                width: Theme.iconSizeMedium + Theme.horizontalPageMargin
                height: parent.height

                Image {
                    anchors.left: parent.left
                    anchors.verticalCenter: parent.verticalCenter
                    source: "image://theme/icon-m-down"
                    width: Theme.iconSizeMedium
                    height: Theme.iconSizeMedium
                    fillMode: Image.PreserveAspectFit
                    opacity: clearArea.pressed ? 1.0 : 0.6
                }

                MouseArea {
                    id: clearArea
                    anchors.fill: parent
                    onClicked: findBar.closeClicked()
                }
            }
        }

        Row {
            id: navRow
            width: parent.width
            height: Theme.itemSizeSmall
            spacing: Theme.paddingSmall

            Item {
                width: (parent.width - prevBtn2.width - counterLabel2.width - nextBtn2.width - Theme.paddingSmall * 2) / 2
                height: parent.height
            }

            IconButton {
                id: prevBtn2
                icon.source: "image://theme/icon-m-back"
                anchors.verticalCenter: parent.verticalCenter
                enabled: findBar.totalMatches > 0
                opacity: enabled ? 1.0 : 0.3
                onClicked: findBar.previousClicked()
            }

            Label {
                id: counterLabel2
                anchors.verticalCenter: parent.verticalCenter
                text: {
                    if (!searchField.text || searchField.text.trim().length === 0) return ""
                    if (findBar.totalMatches === 0) return "0/0"
                    var n = (findBar.currentMatchIndex >= 0 ? findBar.currentMatchIndex : 0) + 1
                    return n + "/" + findBar.totalMatches
                }
                color: findBar.totalMatches > 0 ? Theme.highlightColor : Theme.secondaryColor
                font.pixelSize: Theme.fontSizeSmall
                visible: text.length > 0
                horizontalAlignment: Text.AlignHCenter
                width: implicitWidth + Theme.paddingSmall
            }

            IconButton {
                id: nextBtn2
                icon.source: "image://theme/icon-m-forward"
                anchors.verticalCenter: parent.verticalCenter
                enabled: findBar.totalMatches > 0
                opacity: enabled ? 1.0 : 0.3
                onClicked: findBar.nextClicked()
            }
        }
    }
}
