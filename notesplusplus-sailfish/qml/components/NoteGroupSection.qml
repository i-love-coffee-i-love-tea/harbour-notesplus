import QtQuick 2.6
import Sailfish.Silica 1.0

Item {
    id: groupSection
    width: parent ? parent.width : Screen.width
    height: mainColumn.height
    implicitHeight: mainColumn.height
    visible: (groupPath.length > 0) || (depth > 0) || (pages && pages.length > 0) || (childrenGroups && childrenGroups.length > 0)

    property var groupData: ({})
    property bool isPortraitOrientation: true
    property var remorsePopupRef: null

    property string groupPath: groupData && groupData.path !== undefined ? groupData.path : ""
    property string displayName: {
        if (groupData && groupData.display_name && groupData.display_name.length > 0) {
            return groupData.display_name
        }
        if (groupPath.length > 0) {
            var slash = groupPath.lastIndexOf("/")
            return slash >= 0 ? groupPath.substring(slash + 1) : groupPath
        }
        return qsTr("Recent Pages")
    }
    property bool isCollapsed: groupData && groupData.collapsed !== undefined ? groupData.collapsed : false
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

    function handleGroupMenuAction(action) {
        if (action === "new_page") {
            var newPage = pageStack.push(Qt.resolvedUrl("../pages/NewPageDialog.qml"), {
                targetGroup: groupPath
            })
            newPage.accepted.connect(function() {
                if (newPage.pageName && newPage.pageName.length > 0) {
                    bridge.create_page(newPage.pageName)
                }
            })
        } else if (action === "new_subgroup") {
            var newSub = pageStack.push(Qt.resolvedUrl("../pages/NewGroupDialog.qml"), {
                parentPath: groupPath
            })
            newSub.accepted.connect(function() {
                if (newSub.groupName && newSub.groupName.length > 0) {
                    bridge.create_group(groupPath, newSub.groupName)
                }
            })
        } else if (action === "rename") {
            var ren = pageStack.push(Qt.resolvedUrl("../pages/RenameGroupDialog.qml"), {
                oldPath: groupPath,
                currentName: displayName
            })
            ren.accepted.connect(function() {
                if (ren.newName && ren.newName.length > 0) {
                    bridge.rename_group(groupPath, ren.newName)
                }
            })
        } else if (action === "delete") {
            if (remorsePopupRef) {
                remorsePopupRef.execute(qsTr("Deleting group '%1'").arg(displayName), function() {
                    bridge.delete_group(groupPath, true)
                })
            } else {
                bridge.delete_group(groupPath, true)
            }
        }
    }

    function openGroupMenu() {
        if (!groupPath || groupPath.length === 0) return
        var menuPage = pageStack.push(Qt.resolvedUrl("../pages/GroupMenuDialog.qml"), {
            groupPath: groupPath,
            displayName: displayName
        })
        menuPage.actionSelected.connect(function(action) {
            handleGroupMenuAction(action)
        })
    }

    Column {
        id: mainColumn
        width: parent ? parent.width : Screen.width
        spacing: 0

        // Section Header: left-aligned text, right-aligned action icons
        Item {
            id: headerItem
            width: parent ? parent.width : Screen.width
            visible: groupPath.length > 0 || (depth > 0) || (pages && pages.length > 0)
            height: visible ? Theme.itemSizeSmall : 0

            BackgroundItem {
                id: headerBg
                anchors.fill: parent
                onClicked: {
                    if (groupPath.length > 0) {
                        bridge.toggle_group_collapsed(groupPath)
                    }
                }
                onPressAndHold: {
                    openGroupMenu()
                }
            }

            // Left side: group name + count
            Item {
                anchors.left: parent.left
                anchors.leftMargin: Theme.horizontalPageMargin + (Math.max(0, depth - 1) * Theme.paddingLarge)
                anchors.right: (actionRow.visible && groupPath.length > 0) ? actionRow.left : parent.right
                anchors.rightMargin: (actionRow.visible && groupPath.length > 0) ? Theme.paddingMedium : Theme.horizontalPageMargin
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
                        color: headerBg.highlighted ? Theme.highlightColor : Theme.primaryColor
                        truncationMode: TruncationMode.Fade
                        maximumLineCount: 1
                    }

                    Label {
                        id: countLabel
                        anchors.verticalCenter: parent.verticalCenter
                        text: "(" + (pages ? pages.length : 0) + ")"
                        font.pixelSize: Theme.fontSizeExtraSmall
                        color: Theme.secondaryColor
                        visible: groupPath.length > 0
                    }
                }
            }

            // Right side: add + collapse
            Row {
                id: actionRow
                anchors.right: parent.right
                anchors.rightMargin: Theme.horizontalPageMargin
                anchors.verticalCenter: parent.verticalCenter
                spacing: 0
                visible: groupPath.length > 0

                IconButton {
                    anchors.verticalCenter: parent.verticalCenter
                    icon.source: "image://theme/icon-s-add"
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
                    icon.source: isCollapsed ? "image://theme/icon-s-plus" : "image://theme/icon-s-minus"
                    onClicked: {
                        bridge.toggle_group_collapsed(groupPath)
                    }
                }
            }

            // Bottom border (only when group has content)
            Rectangle {
                anchors.bottom: parent.bottom
                anchors.left: parent.left
                anchors.right: parent.right
                height: 1
                color: Theme.rgba(Theme.primaryColor, 0.15)
                visible: (pages && pages.length > 0) || (childrenGroups && childrenGroups.length > 0)
            }
        }

        // Body content (Notes grid + Nested Child Groups)
        Column {
            id: bodyContent
            width: parent ? parent.width : Screen.width
            spacing: 0
            visible: !isCollapsed

            // Empty group placeholder
            Label {
                width: parent.width - Theme.horizontalPageMargin * 2
                x: Theme.horizontalPageMargin + (Math.max(0, depth - 1) * Theme.paddingLarge) + Theme.paddingLarge
                text: qsTr("Empty group")
                font.italic: true
                font.pixelSize: Theme.fontSizeExtraSmall
                color: Theme.secondaryColor
                visible: groupPath.length > 0 && (!pages || pages.length === 0) && (!childrenGroups || childrenGroups.length === 0)
            }

            // Notes inside this group
            NoteCardGrid {
                id: groupNotesGrid
                width: parent.width
                isPortraitOrientation: groupSection.isPortraitOrientation
                model: pages
                visible: pages && pages.length > 0
                onItemClicked: function(itemData, itemIndex) {
                    var fullPath = itemData.full_path || (itemData.group_path ? itemData.group_path + "/" + itemData.filename : itemData.filename)
                    pageStack.push(Qt.resolvedUrl("../pages/PageView.qml"), {
                        pageName: itemData.name || fullPath
                    })
                    bridge.load_page(fullPath)
                }
            }

            // Child groups (Recursion via Loader in a Repeater)
            Repeater {
                model: childrenGroups
                delegate: Loader {
                    id: childGroupLoader
                    width: parent ? parent.width : Screen.width
                    source: Qt.resolvedUrl("NoteGroupSection.qml")
                    height: item ? item.height : 0
                    visible: item ? item.visible : true

                    onLoaded: {
                        if (item) {
                            item.groupData = modelData
                            item.isPortraitOrientation = groupSection.isPortraitOrientation
                            item.remorsePopupRef = groupSection.remorsePopupRef
                        }
                    }

                    Binding {
                        target: childGroupLoader.item
                        property: "groupData"
                        value: modelData
                        when: childGroupLoader.status === Loader.Ready
                    }
                    Binding {
                        target: childGroupLoader.item
                        property: "isPortraitOrientation"
                        value: groupSection.isPortraitOrientation
                        when: childGroupLoader.status === Loader.Ready
                    }
                    Binding {
                        target: childGroupLoader.item
                        property: "remorsePopupRef"
                        value: groupSection.remorsePopupRef
                        when: childGroupLoader.status === Loader.Ready
                    }
                }
            }
        }
    }
}
