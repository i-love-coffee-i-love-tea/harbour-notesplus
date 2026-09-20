import QtQuick 2.6
import Sailfish.Silica 1.0

Dialog {
    id: sortNotesDialog
    allowedOrientations: Orientation.All

    property string groupPath: ""
    property string groupName: ""
    property string currentSort: "newest"
    property string selectedSort: currentSort === "name" ? "name" : "newest"

    canAccept: true

    Column {
        width: parent.width
        spacing: Theme.paddingMedium

        DialogHeader {
            title: qsTr("Sort Notes")
            acceptText: qsTr("Save")
            cancelText: qsTr("Cancel")
        }

        Label {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.margins: Theme.horizontalPageMargin
            visible: groupName.length > 0
            text: qsTr("Group: %1").arg(groupName)
            color: Theme.highlightColor
            font.pixelSize: Theme.fontSizeSmall
            truncationMode: TruncationMode.Fade
        }

        ComboBox {
            id: sortCombo
            width: parent.width
            label: qsTr("Sort Notes")
            currentIndex: currentSort === "name" ? 1 : 0
            menu: ContextMenu {
                MenuItem { text: qsTr("Newest first") }
                MenuItem { text: qsTr("By name") }
            }
            onCurrentIndexChanged: {
                sortNotesDialog.selectedSort = (currentIndex === 1) ? "name" : "newest"
            }
        }
    }
}
