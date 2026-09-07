import QtQuick 2.6
import Sailfish.Silica 1.0

Dialog {
    id: newPageDialog
    allowedOrientations: Orientation.All

    property string pageName: ""

    canAccept: pageName.length > 0

    Column {
        width: parent.width
        spacing: Theme.paddingMedium

        DialogHeader {
            acceptText: qsTr("Create")
            cancelText: qsTr("Cancel")
        }

        TextField {
            id: nameField
            width: parent.width
            placeholderText: qsTr("Page name")
            label: qsTr("Page name")
            focus: true
            onTextChanged: {
                pageName = text
            }
            EnterKey.enabled: text.length > 0
            EnterKey.iconSource: "image://theme/icon-m-enter-accept"
            EnterKey.onClicked: newPageDialog.accept()
        }
    }
}
