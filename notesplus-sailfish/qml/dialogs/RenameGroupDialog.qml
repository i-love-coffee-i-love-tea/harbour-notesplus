import QtQuick 2.6
import Sailfish.Silica 1.0

Dialog {
    id: renameGroupDialog
    allowedOrientations: Orientation.All

    property string oldPath: ""
    property string currentName: ""
    property string newName: nameField ? nameField.text.trim() : ""

    canAccept: newName.length > 0 && newName !== currentName

    Column {
        width: parent.width
        spacing: Theme.paddingMedium

        DialogHeader {
            title: qsTr("Rename Group")
            acceptText: qsTr("Rename")
            cancelText: qsTr("Cancel")
        }

        TextField {
            id: nameField
            width: parent.width
            text: currentName
            placeholderText: qsTr("New group name")
            label: qsTr("New group name")
            focus: true
            onTextChanged: {
                newName = text
            }
            EnterKey.enabled: newName.trim().length > 0 && newName.trim() !== currentName
            EnterKey.iconSource: "image://theme/icon-m-enter-accept"
            EnterKey.onClicked: renameGroupDialog.accept()
        }
    }
}
