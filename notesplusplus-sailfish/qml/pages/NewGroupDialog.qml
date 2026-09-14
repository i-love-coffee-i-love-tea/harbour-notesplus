import QtQuick 2.6
import Sailfish.Silica 1.0

Dialog {
    id: newGroupDialog
    allowedOrientations: Orientation.All

    property string parentPath: ""
    property string groupName: nameField ? nameField.text.trim() : ""
    property string noteSort: sortCombo ? (sortCombo.currentIndex === 1 ? "name" : "newest") : "newest"

    canAccept: groupName.length > 0

    Column {
        width: parent.width
        spacing: Theme.paddingMedium

        DialogHeader {
            title: parentPath.length > 0 ? qsTr("New Subgroup") : qsTr("New Group")
            acceptText: qsTr("Create")
            cancelText: qsTr("Cancel")
        }

        Label {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.margins: Theme.horizontalPageMargin
            visible: parentPath.length > 0
            text: qsTr("Inside: %1").arg(parentPath)
            color: Theme.highlightColor
            font.pixelSize: Theme.fontSizeSmall
        }

        TextField {
            id: nameField
            width: parent.width
            placeholderText: qsTr("Group name")
            label: qsTr("Group name")
            focus: true
            onTextChanged: {
                groupName = text
            }
            EnterKey.enabled: text.trim().length > 0
            EnterKey.iconSource: "image://theme/icon-m-enter-accept"
            EnterKey.onClicked: newGroupDialog.accept()
        }

        ComboBox {
            id: sortCombo
            width: parent.width
            label: qsTr("Sort Notes")
            currentIndex: 0
            menu: ContextMenu {
                MenuItem { text: qsTr("Newest first") }
                MenuItem { text: qsTr("By name") }
            }
        }
    }
}
