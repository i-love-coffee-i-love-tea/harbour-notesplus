import QtQuick 2.6
import Sailfish.Silica 1.0

Dialog {
    id: confirmDialog
    allowedOrientations: Orientation.All

    property string title: qsTr("Confirm")
    property string message: ""
    property string acceptText: qsTr("Yes")
    property string cancelText: qsTr("Cancel")

    canAccept: true

    Column {
        width: parent.width
        spacing: Theme.paddingLarge

        DialogHeader {
            title: confirmDialog.title
            acceptText: confirmDialog.acceptText
            cancelText: confirmDialog.cancelText
        }

        Label {
            x: Theme.horizontalPageMargin
            width: parent.width - Theme.horizontalPageMargin * 2
            text: confirmDialog.message
            color: Theme.highlightColor
            font.pixelSize: Theme.fontSizeMedium
            wrapMode: Text.Wrap
        }
    }
}
