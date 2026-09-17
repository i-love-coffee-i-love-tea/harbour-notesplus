import QtQuick 2.6
import Sailfish.Silica 1.0
import "../js/ThemeColors.js" as TC

GridItem {
    id: noteCardItem

    contentHeight: _cellHeight
    property real _cellHeight: width > 0 ? width : Theme.itemSizeExtraLarge

    property var cardData: ({})
    property int noteIndex: 0

    property string _fullPath: (cardData && cardData.full_path) ? cardData.full_path : ((cardData && cardData.group_path ? cardData.group_path + "/" : "") + (cardData ? cardData.filename : ""))
    property string _title: (cardData && cardData.name) ? cardData.name : _fullPath
    property string _group: (cardData && cardData.group_path) ? cardData.group_path : ""
    property real _previewPadding: width < Theme.itemSizeSmall ? Theme.paddingSmall : (width < Theme.itemSizeMedium ? Math.round(Theme.paddingMedium * 0.7) : Theme.paddingMedium)

    menu: ContextMenu {
        MenuItem {
            text: qsTr("Move")
            onClicked: {
                var dialog = pageStack.push(Qt.resolvedUrl("../pages/MovePageDialog.qml"), {
                    pageFullPath: _fullPath,
                    pageTitle: _title,
                    currentGroup: _group
                })
                dialog.accepted.connect(function() {
                    var target = dialog.targetGroup
                    if (target !== _group) {
                        bridge.move_page_to_group(_fullPath, target)
                    }
                })
            }
        }
        MenuItem {
            text: qsTr("Rename")
            onClicked: {
                var dialog = pageStack.push(Qt.resolvedUrl("../pages/RenamePageDialog.qml"), {
                    currentTitle: _title
                })
                dialog.accepted.connect(function() {
                    var newTitle = dialog.newTitle
                    if (newTitle.length > 0 && newTitle !== _title) {
                        bridge.rename_page(_fullPath, newTitle)
                    }
                })
            }
        }
        MenuItem {
            text: qsTr("Delete")
            onClicked: {
                bridge.delete_page(_fullPath)
            }
        }
    }

    function getNoteColor(name) {
        var palette = TC.kNoteCardPalette;
        var hash = 0;
        if (name) {
            for (var i = 0; i < name.length; i++) {
                hash = (hash * 31 + name.charCodeAt(i)) & 0x7FFFFFFF;
            }
        }
        return palette[hash % palette.length];
    }

    Rectangle {
        id: cardBox
        width: parent.width
        height: noteCardItem.contentHeight
        color: noteCardItem.highlighted ? Theme.rgba(Theme.highlightBackgroundColor, 0.22) : Theme.rgba(Theme.highlightBackgroundColor, 0.05)
        border.color: noteCardItem.highlighted ? Theme.highlightColor : Theme.rgba(Theme.primaryColor, 0.12)
        border.width: 1

        MiniDocPreview {
            id: miniDocPreview
            anchors {
                left: parent.left
                right: parent.right
                top: parent.top
                bottom: cardFooter.top
                leftMargin: _previewPadding
                rightMargin: _previewPadding
                topMargin: _previewPadding
                bottomMargin: 0
            }
            showBorder: false
            previewBlocks: (noteCardItem.cardData && noteCardItem.cardData.preview_blocks) ? noteCardItem.cardData.preview_blocks : []
            previewBlocksJson: noteCardItem.cardData ? (noteCardItem.cardData.preview_blocks_json || "") : ""
            snippet: noteCardItem.cardData ? (noteCardItem.cardData.snippet || "") : ""
            highlighted: noteCardItem.highlighted
        }

        Item {
            id: cardFooter
            anchors {
                left: parent.left
                right: parent.right
                bottom: parent.bottom
                leftMargin: _previewPadding
                rightMargin: _previewPadding
                bottomMargin: Theme.paddingSmall
            }
            height: (width < Theme.itemSizeSmall ? Theme.paddingMedium : Theme.paddingLarge) + Theme.paddingMedium

            Rectangle {
                id: colorBar
                anchors {
                    left: parent.left
                    leftMargin: Theme.horizontalPageMargin * miniDocPreview.contentScale
                    verticalCenter: parent.verticalCenter
                }
                width: Math.round(Theme.itemSizeExtraSmall * 1.2)
                height: Theme.paddingSmall
                radius: Math.round(Theme.paddingSmall / 2)
                color: (noteCardItem.cardData && noteCardItem.cardData.color) ? noteCardItem.cardData.color : noteCardItem.getNoteColor(noteCardItem.cardData ? noteCardItem.cardData.name : "")
            }

            Label {
                id: indexLabel
                anchors {
                    right: parent.right
                    verticalCenter: parent.verticalCenter
                }
                text: (noteCardItem.cardData && noteCardItem.cardData.id) ? noteCardItem.cardData.id.toString() : ""
                font.pixelSize: Theme.fontSizeSmall
                color: Theme.secondaryColor
            }
        }
    }
}
