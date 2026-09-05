import QtQuick 2.0
import Sailfish.Silica 1.0

Rectangle {
    id: undoBannerRoot
    width: parent.width
    height: Theme.itemSizeSmall
    color: Theme.rgba(Theme.highlightBackgroundColor, 0.35)
    border.color: Theme.highlightColor
    border.width: 1
    radius: Theme.paddingSmall
    clip: true

    signal undoTriggered()

    Row {
        anchors.fill: parent
        anchors.leftMargin: Theme.paddingMedium
        anchors.rightMargin: Theme.paddingMedium
        spacing: Theme.paddingSmall

        Icon {
            source: "image://theme/icon-m-backup"
            anchors.verticalCenter: parent.verticalCenter
        }

        Label {
            text: "Note modified by AI"
            font.pixelSize: Theme.fontSizeExtraSmall
            color: Theme.primaryColor
            anchors.verticalCenter: parent.verticalCenter
            width: parent.width - undoButton.width - Theme.paddingMedium * 3
            truncationMode: TruncationMode.Fade
        }

        Button {
            id: undoButton
            text: "Undo"
            preferredWidth: Theme.buttonWidthExtraSmall
            anchors.verticalCenter: parent.verticalCenter
            onClicked: {
                undoBannerRoot.undoTriggered()
            }
        }
    }
}
