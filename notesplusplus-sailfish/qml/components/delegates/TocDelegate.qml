import QtQuick 2.6
import Sailfish.Silica 1.0
import "../"
import "../../js/BlockHtmlUtils.js" as BlockHtmlUtils

Item {
    id: tocDelegateItem
    property var blockData: ({})
    property var allBlocks: []
    property int blockIndex: -1
    property bool isTocCollapsed: false
    property bool interactive: true

    signal toggleToc(int blockIndex)
    signal jumpToBlock(int targetIndex)

    anchors.left: parent ? parent.left : undefined
    anchors.right: parent ? parent.right : undefined
    anchors.leftMargin: Theme.horizontalPageMargin
    anchors.rightMargin: Theme.horizontalPageMargin
    height: tocRect.height

    Rectangle {
        id: tocRect
        anchors.left: parent.left
        anchors.right: parent.right
        height: tocColumn.height + Theme.paddingMedium * 2
        color: Qt.rgba(Theme.highlightBackgroundColor.r, Theme.highlightBackgroundColor.g, Theme.highlightBackgroundColor.b, 0.1)
        border.color: Theme.highlightColor
        border.width: 1
        radius: 0
        clip: true

        Column {
            id: tocColumn
            anchors {
                left: parent.left
                right: parent.right
                margins: Theme.paddingMedium
                top: parent.top
                topMargin: Theme.paddingMedium
            }
            spacing: Theme.paddingSmall

            BackgroundItem {
                id: tocHeader
                width: parent.width
                height: Math.max(Theme.itemSizeExtraSmall * 0.7, titleLabel.height + Theme.paddingSmall * 2)
                enabled: tocDelegateItem.interactive

                Label {
                    id: titleLabel
                    anchors.left: parent.left
                    anchors.right: chevronIcon.left
                    anchors.rightMargin: Theme.paddingSmall
                    anchors.verticalCenter: parent.verticalCenter
                    text: "Table of Contents"
                    font.bold: true
                    font.family: (typeof app !== "undefined" && app && app.docFontFamily && app.docFontFamily.length > 0) ? app.docFontFamily : Theme.fontFamily
                    font.pixelSize: Math.round(Theme.fontSizeSmall * ((typeof app !== "undefined" && app && app.fontScale) ? app.fontScale : 1.0))
                    color: tocHeader.highlighted ? Theme.primaryColor : Theme.highlightColor
                    truncationMode: TruncationMode.Fade
                }

                Image {
                    id: chevronIcon
                    anchors.right: parent.right
                    anchors.verticalCenter: parent.verticalCenter
                    source: "image://theme/icon-s-down?" + (tocHeader.highlighted ? Theme.primaryColor : Theme.highlightColor)
                    transformOrigin: Item.Center
                    rotation: tocDelegateItem.isTocCollapsed ? -90 : 0
                    Behavior on rotation { NumberAnimation { duration: 150 } }
                }

                onClicked: {
                    if (tocDelegateItem.interactive) {
                        tocDelegateItem.toggleToc(tocDelegateItem.blockIndex)
                    }
                }
            }

            Column {
                id: entriesColumn
                width: parent.width
                spacing: 2
                visible: !tocDelegateItem.isTocCollapsed

                property var tocList: {
                    if (tocDelegateItem.blockData && tocDelegateItem.blockData.headings && tocDelegateItem.blockData.headings.length > 0) {
                        return tocDelegateItem.blockData.headings
                    }
                    var headings = []
                    var blocks = tocDelegateItem.allBlocks || []
                    for (var i = 0; i < blocks.length; i++) {
                        var b = blocks[i]
                        if (b && b.type === "heading" && b.level >= 1 && b.level <= 5) {
                            headings.push({
                                level: b.level,
                                text: BlockHtmlUtils.spansToPlainText(b.spans || []),
                                index: i
                            })
                        }
                    }
                    return headings
                }

                property int minLevel: {
                    var minL = 99
                    for (var i = 0; i < tocList.length; i++) {
                        if (tocList[i] && tocList[i].level && tocList[i].level < minL) {
                            minL = tocList[i].level
                        }
                    }
                    return minL === 99 ? 1 : minL
                }

                Repeater {
                    id: tocRepeater
                    model: entriesColumn.tocList

                    delegate: BackgroundItem {
                        id: tocEntry
                        property var headingItem: modelData || ({})
                        property int headingLevel: headingItem && headingItem.level ? headingItem.level : 1
                        property string headingText: headingItem && headingItem.text ? headingItem.text : ""
                        property int headingIndex: headingItem && headingItem.index !== undefined ? headingItem.index : -1
                        property int indent: Math.max(0, (headingLevel - entriesColumn.minLevel)) * Theme.paddingLarge
                        width: parent.width
                        height: headingLabel.height + Theme.paddingSmall * 2
                        enabled: tocDelegateItem.interactive

                        Label {
                            id: headingLabel
                            anchors.left: parent.left
                            anchors.right: parent.right
                            anchors.leftMargin: indent
                            anchors.rightMargin: Theme.paddingSmall
                            anchors.verticalCenter: parent.verticalCenter
                            text: "• " + headingText
                            font.family: (typeof app !== "undefined" && app && app.docFontFamily && app.docFontFamily.length > 0) ? app.docFontFamily : Theme.fontFamily
                            font.pixelSize: Math.round(Theme.fontSizeSmall * ((typeof app !== "undefined" && app && app.fontScale) ? app.fontScale : 1.0))
                            color: tocEntry.highlighted ? Theme.primaryColor : Theme.highlightColor
                            wrapMode: Text.Wrap
                        }

                        onClicked: {
                            if (tocDelegateItem.interactive && headingIndex >= 0) {
                                tocDelegateItem.jumpToBlock(headingIndex)
                            }
                        }
                    }
                }
            }
        }
    }
}
