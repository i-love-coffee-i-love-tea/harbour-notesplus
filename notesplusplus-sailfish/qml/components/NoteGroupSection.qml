import QtQuick 2.6
import Sailfish.Silica 1.0

Column {
    id: groupSection
    width: parent ? parent.width : Screen.width
    spacing: 0
    visible: (groupPath.length > 0) || (depth > 0) || (pages && pages.length > 0) || (childrenGroups && childrenGroups.length > 0)

    property var groupData: ({})
    property bool isPortraitOrientation: true
    property var remorsePopupRef: null

    property string groupPath: groupData && groupData.path !== undefined ? groupData.path : ""
    property string noteSort: groupData && groupData.note_sort ? groupData.note_sort : "newest"
    property string displayName: {
        if (groupData && groupData.display_name && groupData.display_name.length > 0) {
            return groupData.display_name
        }
        if (groupPath.length > 0) {
            var slash = groupPath.lastIndexOf("/")
            return slash >= 0 ? groupPath.substring(slash + 1) : groupPath
        }
        return qsTr("Recent Notes")
    }
    property bool isCollapsed: groupData && groupData.collapsed !== undefined ? groupData.collapsed : false
    onGroupDataChanged: {
        if (groupData && groupData.collapsed !== undefined) {
            isCollapsed = groupData.collapsed
        }
    }
    property int depth: {
        if (groupData && groupData.depth !== undefined && groupData.depth > 0) {
            return groupData.depth
        }
        if (groupPath.length > 0) {
            return groupPath.split("/").length
        }
        return 0
    }
    property var pages: groupData && groupData.pages ? groupData.pages : []
    property var childrenGroups: groupData && groupData.children ? groupData.children : []

    function toggleCollapsed() {
        isCollapsed = !isCollapsed
        bridge.toggle_group_collapsed(groupPath)
    }

    // Section Header with Context Menu
    ListItem {
        id: headerItem
        width: parent.width
        contentHeight: Theme.itemSizeSmall
        visible: (groupPath.length > 0) || (depth > 0) || (pages && pages.length > 0) || (childrenGroups && childrenGroups.length > 0)

        onClicked: {
            toggleCollapsed()
        }

        menu: ContextMenu {
            MenuItem {
                text: qsTr("New Note")
                onClicked: {
                    var newPage = pageStack.push(Qt.resolvedUrl("../pages/NewPageDialog.qml"), {
                        targetGroup: groupPath
                    })
                    newPage.accepted.connect(function() {
                        if (newPage.pageName && newPage.pageName.length > 0) {
                            bridge.create_page(newPage.pageName)
                        }
                    })
                }
            }

            MenuItem {
                text: qsTr("New Subgroup")
                onClicked: {
                    var newSub = pageStack.push(Qt.resolvedUrl("../pages/NewGroupDialog.qml"), {
                        parentPath: groupPath
                    })
                    newSub.accepted.connect(function() {
                        if (newSub.groupName && newSub.groupName.length > 0) {
                            if (bridge.create_group(groupPath, newSub.groupName)) {
                                if (newSub.noteSort && newSub.noteSort !== "newest") {
                                    var fullChildPath = groupPath.length > 0 ? (groupPath + "/" + newSub.groupName) : newSub.groupName
                                    bridge.set_group_note_sort(fullChildPath, newSub.noteSort)
                                }
                            }
                        }
                    })
                }
            }

            MenuItem {
                text: noteSort === "name" ? qsTr("Sort Notes: Newest First") : qsTr("Sort Notes: By Name")
                onClicked: {
                    var nextSort = (noteSort === "name") ? "newest" : "name"
                    bridge.set_group_note_sort(groupPath, nextSort)
                }
            }

            MenuItem {
                text: qsTr("Rename")
                visible: groupPath.length > 0
                onClicked: {
                    var ren = pageStack.push(Qt.resolvedUrl("../pages/RenameGroupDialog.qml"), {
                        oldPath: groupPath,
                        currentName: displayName
                    })
                    ren.accepted.connect(function() {
                        if (ren.newName && ren.newName.length > 0) {
                            bridge.rename_group(groupPath, ren.newName)
                        }
                    })
                }
            }

            MenuItem {
                text: qsTr("Delete")
                visible: groupPath.length > 0
                onClicked: {
                    if (remorsePopupRef) {
                        remorsePopupRef.execute(qsTr("Deleting group '%1'").arg(displayName), function() {
                            bridge.delete_group(groupPath, true)
                        })
                    } else {
                        bridge.delete_group(groupPath, true)
                    }
                }
            }
        }

        Item {
            id: headerContent
            width: parent.width
            height: headerItem.contentHeight

            Item {
                anchors.left: parent.left
                anchors.leftMargin: Theme.horizontalPageMargin + (Math.max(0, depth - 1) * Theme.paddingLarge)
                anchors.right: actionRow.left
                anchors.rightMargin: Theme.paddingMedium
                anchors.verticalCenter: parent.verticalCenter
                height: parent.height

                Row {
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: Theme.paddingSmall

                    Label {
                        anchors.verticalCenter: parent.verticalCenter
                        text: displayName
                        font.pixelSize: depth <= 1 ? Theme.fontSizeMedium : Theme.fontSizeSmall
                        font.bold: depth <= 1
                        color: headerItem.highlighted ? Theme.highlightColor : Theme.primaryColor
                        truncationMode: TruncationMode.Fade
                        maximumLineCount: 1
                    }

                    Label {
                        id: countLabel
                        anchors.verticalCenter: parent.verticalCenter
                        text: "(" + (pages ? pages.length : 0) + ")"
                        font.pixelSize: Theme.fontSizeExtraSmall
                        color: Theme.secondaryColor
                    }
                }
            }

            Row {
                id: actionRow
                anchors.right: parent.right
                anchors.rightMargin: Theme.horizontalPageMargin
                anchors.verticalCenter: parent.verticalCenter
                spacing: 0

                IconButton {
                    anchors.verticalCenter: parent.verticalCenter
                    icon.source: "image://theme/icon-m-add"
                    icon.width: Theme.iconSizeSmall
                    icon.height: Theme.iconSizeSmall
                    icon.color: Theme.primaryColor
                    width: Theme.itemSizeExtraSmall
                    height: Theme.itemSizeExtraSmall
                    onClicked: {
                        var dialog = pageStack.push(Qt.resolvedUrl("../pages/NewPageDialog.qml"), {
                            targetGroup: groupPath
                        })
                        dialog.accepted.connect(function() {
                            if (dialog.pageName.length > 0) {
                                bridge.create_page(dialog.pageName)
                            }
                        })
                    }
                }

                IconButton {
                    anchors.verticalCenter: parent.verticalCenter
                    icon.source: isCollapsed ? "image://theme/icon-m-right" : "image://theme/icon-m-down"
                    icon.width: Theme.iconSizeSmall
                    icon.height: Theme.iconSizeSmall
                    icon.color: Theme.primaryColor
                    width: Theme.itemSizeExtraSmall
                    height: Theme.itemSizeExtraSmall
                    onClicked: {
                        toggleCollapsed()
                    }
                }
            }

            Rectangle {
                anchors.bottom: parent.bottom
                anchors.left: parent.left
                anchors.right: parent.right
                height: 1
                color: Theme.rgba(Theme.primaryColor, 0.15)
                visible: (pages && pages.length > 0) || (childrenGroups && childrenGroups.length > 0)
            }
        }
    }

    // Empty group placeholder
    Label {
        width: parent.width - Theme.horizontalPageMargin * 2
        x: Theme.horizontalPageMargin + (Math.max(0, depth - 1) * Theme.paddingLarge) + Theme.paddingLarge
        text: qsTr("Empty group")
        font.italic: true
        font.pixelSize: Theme.fontSizeExtraSmall
        color: Theme.secondaryColor
        visible: !isCollapsed && groupPath.length > 0 && (!pages || pages.length === 0) && (!childrenGroups || childrenGroups.length === 0)
    }

    // Notes grid
    NoteCardGrid {
        id: groupNotesGrid
        width: parent.width
        isPortraitOrientation: groupSection.isPortraitOrientation
        model: pages
        visible: !isCollapsed && pages && pages.length > 0
        onItemClicked: function(itemData, itemIndex) {
            var fullPath = itemData.full_path || (itemData.group_path ? itemData.group_path + "/" + itemData.filename : itemData.filename)
            pageStack.push(Qt.resolvedUrl("../pages/PageView.qml"), {
                initialTargetPage: fullPath,
                pageName: itemData.name || fullPath
            })
            bridge.load_page(fullPath)
        }
    }

    // Child groups — Loader as direct delegate (no wrapper Item)
    Repeater {
        id: childGroupRepeater
        model: !isCollapsed ? childrenGroups : []
        delegate: Loader {
            id: childLoader
            width: parent ? parent.width : Screen.width
            height: item ? Math.max(item.implicitHeight, 1) : 0
            visible: true
            clip: false
            source: Qt.resolvedUrl("NoteGroupSection.qml")

            onLoaded: {
                if (item) {
                    item.groupData = modelData
                    item.isPortraitOrientation = groupSection.isPortraitOrientation
                    item.remorsePopupRef = groupSection.remorsePopupRef
                }
            }

            Binding {
                target: childLoader.item
                property: "groupData"
                value: modelData
                when: childLoader.status === Loader.Ready
            }
            Binding {
                target: childLoader.item
                property: "isPortraitOrientation"
                value: groupSection.isPortraitOrientation
                when: childLoader.status === Loader.Ready
            }
            Binding {
                target: childLoader.item
                property: "remorsePopupRef"
                value: groupSection.remorsePopupRef
                when: childLoader.status === Loader.Ready
            }
        }
    }
}
