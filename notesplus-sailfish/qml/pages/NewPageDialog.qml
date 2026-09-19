import QtQuick 2.6
import Sailfish.Silica 1.0
import "../js/ThemeColors.js" as TC

Dialog {
    id: newPageDialog
    allowedOrientations: Orientation.All

    property string targetGroup: ""
    property var parsedGroups: []
    property string selectedColor: TC.randomNoteColor()

    property string pageName: {
        var baseName = nameField ? nameField.text.trim() : ""
        if (baseName.length === 0) return ""
        var inlineGroup = inlineNewGroupField ? inlineNewGroupField.text.trim() : ""
        if (inlineGroup.length > 0) {
            return inlineGroup + "/" + baseName
        }
        var selectedGroup = targetGroup
        if (groupCombo && groupCombo.currentIndex >= 0 && groupCombo.currentIndex < parsedGroups.length) {
            selectedGroup = parsedGroups[groupCombo.currentIndex].path
        }
        if (selectedGroup && selectedGroup.length > 0) {
            return selectedGroup + "/" + baseName
        }
        return baseName
    }

    Component.onCompleted: {
        var groups = [{ path: "", display_name: qsTr("Root (Ungrouped)") }]
        var matchIndex = 0
        try {
            var raw = bridge.get_groups_json()
            var list = JSON.parse(raw)
            for (var i = 0; i < list.length; i++) {
                groups.push(list[i])
                if (targetGroup.length > 0 && list[i].path === targetGroup) {
                    matchIndex = groups.length - 1
                }
            }
            if (matchIndex === 0 && targetGroup.length > 0) {
                groups.push({ path: targetGroup, display_name: targetGroup })
                matchIndex = groups.length - 1
            }
        } catch(e) {}
        parsedGroups = groups
        groupCombo.currentIndex = matchIndex
    }

    canAccept: nameField ? nameField.text.trim().length > 0 : false

    Column {
        width: parent.width
        spacing: Theme.paddingMedium

        DialogHeader {
            title: targetGroup.length > 0 ? qsTr("New Note in %1").arg(targetGroup) : qsTr("New Note")
            acceptText: qsTr("Create")
            cancelText: qsTr("Cancel")
        }

        TextField {
            id: nameField
            width: parent.width
            placeholderText: qsTr("Page name")
            label: qsTr("Page name")
            focus: true
            EnterKey.enabled: text.trim().length > 0
            EnterKey.iconSource: "image://theme/icon-m-enter-accept"
            EnterKey.onClicked: newPageDialog.accept()
        }

        ComboBox {
            id: groupCombo
            width: parent.width
            label: qsTr("Group")
            visible: newPageDialog.parsedGroups.length > 1
            menu: ContextMenu {
                Repeater {
                    model: newPageDialog.parsedGroups
                    MenuItem {
                        text: modelData.path.length > 0 ? (modelData.display_name && modelData.display_name !== modelData.path ? modelData.path + " (" + modelData.display_name + ")" : modelData.path) : (modelData.display_name || qsTr("Root (Ungrouped)"))
                    }
                }
            }
        }

        TextField {
            id: inlineNewGroupField
            width: parent.width
            placeholderText: qsTr("Or enter new group name...")
            label: qsTr("New group folder")
        }

        SectionHeader {
            text: qsTr("Note Color")
        }

        Grid {
            columns: 6
            spacing: Theme.paddingMedium
            anchors.horizontalCenter: parent.horizontalCenter

            Repeater {
                model: TC.kNoteCardPalette
                Rectangle {
                    width: Theme.itemSizeExtraSmall * 0.75
                    height: width
                    radius: width / 2
                    color: modelData
                    border.width: newPageDialog.selectedColor === modelData ? 3 : 1
                    border.color: newPageDialog.selectedColor === modelData ? Theme.highlightColor : Theme.rgba(Theme.primaryColor, 0.25)

                    Rectangle {
                        anchors.centerIn: parent
                        width: parent.width * 0.4
                        height: width
                        radius: width / 2
                        color: Theme.highlightColor
                        visible: newPageDialog.selectedColor === modelData
                    }

                    MouseArea {
                        anchors.fill: parent
                        onClicked: {
                            if (newPageDialog.selectedColor === modelData) {
                                newPageDialog.selectedColor = ""
                            } else {
                                newPageDialog.selectedColor = modelData
                            }
                        }
                    }
                }
            }
        }
    }
}
