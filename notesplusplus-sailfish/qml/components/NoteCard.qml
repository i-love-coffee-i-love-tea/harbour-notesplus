import QtQuick 2.6
import Sailfish.Silica 1.0

BackgroundItem {
    id: noteCardItem

    property var cardData: ({})
    property int noteIndex: 0

    onPressAndHold: {
        var fullPath = (cardData && cardData.full_path) ? cardData.full_path : ((cardData && cardData.group_path ? cardData.group_path + "/" : "") + (cardData ? cardData.filename : ""))
        var title = (cardData && cardData.name) ? cardData.name : fullPath
        var group = (cardData && cardData.group_path) ? cardData.group_path : ""
        var dialog = pageStack.push(Qt.resolvedUrl("../pages/MovePageDialog.qml"), {
            pageFullPath: fullPath,
            pageTitle: title,
            currentGroup: group
        })
        dialog.accepted.connect(function() {
            var target = dialog.targetGroup
            if (target !== group) {
                bridge.move_page_to_group(fullPath, target)
            }
        })
    }

    function getNoteColor(name) {
        var palette = [
            "#e67e22", "#3498db", "#2ecc71", "#9b59b6",
            "#f1c40f", "#e74c3c", "#1abc9c", "#e84393",
            "#00cec9", "#6c5ce7", "#fdcb6e", "#00b894"
        ];
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
        anchors.fill: parent
        color: noteCardItem.highlighted ? Theme.rgba(Theme.highlightBackgroundColor, 0.22) : Theme.rgba(Theme.highlightBackgroundColor, 0.05)
        border.color: noteCardItem.highlighted ? Theme.highlightColor : Theme.rgba(Theme.primaryColor, 0.12)
        border.width: 1

        MiniDocPreview {
            anchors {
                left: parent.left
                right: parent.right
                top: parent.top
                bottom: cardFooter.top
                leftMargin: Theme.paddingMedium
                rightMargin: Theme.paddingMedium
                topMargin: Theme.paddingMedium
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
                leftMargin: Theme.paddingMedium
                rightMargin: Theme.paddingMedium
                bottomMargin: Theme.paddingSmall
            }
            height: Theme.paddingLarge

            Rectangle {
                id: colorBar
                anchors {
                    left: parent.left
                    verticalCenter: parent.verticalCenter
                }
                width: Math.round(Theme.itemSizeExtraSmall * 0.6)
                height: Theme.paddingSmall
                radius: Math.round(Theme.paddingSmall / 2)
                color: noteCardItem.getNoteColor(noteCardItem.cardData ? noteCardItem.cardData.name : "")
            }

            Label {
                id: indexLabel
                anchors {
                    right: parent.right
                    verticalCenter: parent.verticalCenter
                }
                text: (noteCardItem.noteIndex + 1).toString()
                font.pixelSize: Theme.fontSizeExtraSmall
                color: Theme.secondaryColor
            }
        }
    }
}
