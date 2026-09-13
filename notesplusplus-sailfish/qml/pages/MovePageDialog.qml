import QtQuick 2.6
import Sailfish.Silica 1.0

Dialog {
    id: movePageDialog
    allowedOrientations: Orientation.All

    property string pageFullPath: ""
    property string pageTitle: ""
    property string currentGroup: ""
    property string targetGroup: ""

    property var parsedGroups: []

    Component.onCompleted: {
        try {
            var raw = bridge.get_groups_json()
            var list = JSON.parse(raw)
            var groups = [{ path: "", display_name: qsTr("Root (Ungrouped)") }]
            for (var i = 0; i < list.length; i++) {
                groups.push(list[i])
            }
            parsedGroups = groups
        } catch(e) {
            parsedGroups = [{ path: "", display_name: qsTr("Root (Ungrouped)") }]
        }
    }

    canAccept: true

    Column {
        width: parent.width
        spacing: Theme.paddingMedium

        DialogHeader {
            title: qsTr("Move Note")
            acceptText: qsTr("Move")
        }

        Label {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.margins: Theme.horizontalPageMargin
            text: qsTr("Move '%1' to:").arg(pageTitle || pageFullPath)
            color: Theme.highlightColor
            wrapMode: Text.Wrap
        }

        ComboBox {
            id: groupPicker
            width: parent.width
            label: qsTr("Target Group")
            menu: ContextMenu {
                Repeater {
                    model: movePageDialog.parsedGroups
                    MenuItem {
                        text: modelData.display_name ? (modelData.path.length > 0 ? modelData.path : modelData.display_name) : qsTr("Root (Ungrouped)")
                    }
                }
            }
            onCurrentIndexChanged: {
                if (currentIndex >= 0 && currentIndex < movePageDialog.parsedGroups.length) {
                    movePageDialog.targetGroup = movePageDialog.parsedGroups[currentIndex].path
                }
            }
        }
    }
}
