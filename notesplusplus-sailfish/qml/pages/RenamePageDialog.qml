import QtQuick 2.6
import Sailfish.Silica 1.0

Dialog {
    id: renamePageDialog
    allowedOrientations: Orientation.All

    property string currentTitle: ""
    property string newTitle: nameField.text.trim()

    Column {
        width: parent.width

        DialogHeader {
            acceptText: qsTr("Rename")
        }

        TextField {
            id: nameField
            width: parent.width
            label: qsTr("Note title")
            text: currentTitle
            focus: true
            EnterKey.enabled: text.trim().length > 0
            EnterKey.iconSource: "image://theme/icon-m-enter-accept"
            onEnterKeyClicked: renamePageDialog.accept()
        }
    }

    onOpened: nameField.forceActiveFocus()
}
