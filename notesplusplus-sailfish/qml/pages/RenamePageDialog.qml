import QtQuick 2.6
import Sailfish.Silica 1.0

Dialog {
    id: renamePageDialog
    allowedOrientations: Orientation.All

    property string currentTitle: ""
    property string newTitle: ""

    canAccept: newTitle.length > 0 && newTitle !== currentTitle

    Column {
        width: parent.width
        spacing: Theme.paddingMedium

        DialogHeader {
            title: qsTr("Rename Note")
            acceptText: qsTr("Rename")
            cancelText: qsTr("Cancel")
        }

        TextField {
            id: nameField
            width: parent.width
            placeholderText: qsTr("New note title")
            label: qsTr("New note title")
            text: currentTitle
            focus: true
            onTextChanged: {
                newTitle = text.trim()
            }
            EnterKey.enabled: newTitle.length > 0 && newTitle !== currentTitle
            EnterKey.iconSource: "image://theme/icon-m-enter-accept"
            EnterKey.onClicked: renamePageDialog.accept()
        }
    }

    onOpened: {
        nameField.forceActiveFocus()
        nameField.selectAll()
    }
}
