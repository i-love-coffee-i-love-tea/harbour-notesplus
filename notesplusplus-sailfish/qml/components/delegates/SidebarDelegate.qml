import QtQuick 2.6
import Sailfish.Silica 1.0
import "../"
import "../../js/BlockHtmlUtils.js" as BlockHtmlUtils

Item {
    id: sidebarDelegate
    property var blockData: ({})
    property int blockIndex: -1
    property string blockType: blockData && blockData.type ? blockData.type : "sidebar"
    property bool isSidebar: blockType === "sidebar"
    property bool isExample: blockType === "example"
    property bool isOpen: blockType === "open"

    signal xrefActivated(string target)
    signal checkboxToggled(int blockIndex, string itemPath)

    anchors.left: parent ? parent.left : undefined
    anchors.right: parent ? parent.right : undefined
    anchors.leftMargin: Theme.horizontalPageMargin
    anchors.rightMargin: Theme.horizontalPageMargin
    height: containerRect.height

    Rectangle {
        id: containerRect
        anchors.left: parent.left
        anchors.right: parent.right
        height: contentCol.height + Theme.paddingMedium * 2
        color: isSidebar ? Theme.rgba(Theme.highlightBackgroundColor, 0.08) : (isExample ? Theme.rgba(Theme.primaryColor, 0.03) : "transparent")
        border.color: isSidebar ? Theme.rgba(Theme.highlightColor, 0.35) : (isExample ? Theme.rgba(Theme.primaryColor, 0.25) : "transparent")
        border.width: isOpen ? 0 : 1
        radius: 0

        Rectangle {
            visible: isSidebar
            anchors.left: parent.left
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            width: 4
            color: Theme.highlightColor
            radius: 0
        }

        Column {
            id: contentCol
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: parent.top
            anchors.leftMargin: isSidebar ? (Theme.paddingMedium + 6) : (isOpen ? 0 : Theme.paddingMedium)
            anchors.rightMargin: isOpen ? 0 : Theme.paddingMedium
            anchors.topMargin: isOpen ? 0 : Theme.paddingMedium
            spacing: Theme.paddingSmall

            Row {
                width: parent.width
                spacing: Theme.paddingSmall
                visible: Boolean(isSidebar && blockData && ((blockData.title_spans && blockData.title_spans.length > 0) || (blockData.title && blockData.title.length > 0)))

                Rectangle {
                    width: Theme.paddingSmall
                    height: Theme.paddingSmall
                    radius: Theme.paddingSmall / 2
                    color: Theme.highlightColor
                    anchors.verticalCenter: parent.verticalCenter
                }

                InlineText {
                    width: parent.width - Theme.paddingSmall * 2
                    spans: (blockData && blockData.title_spans && blockData.title_spans.length > 0) ? blockData.title_spans : ((blockData && blockData.title) ? [{ type: "text", value: blockData.title }] : undefined)
                    color: Theme.highlightColor
                    font.bold: true
                    font.family: (typeof app !== "undefined" && app && app.docFontFamily && app.docFontFamily.length > 0) ? app.docFontFamily : Theme.fontFamily
                    font.pixelSize: Math.round(Theme.fontSizeSmall * ((typeof app !== "undefined" && app && app.fontScale) ? app.fontScale : 1.0))
                    font.capitalization: Font.AllUppercase
                    wrapMode: Text.WrapAtWordBoundaryOrAnywhere
                }
            }

            InlineText {
                visible: Boolean(!isSidebar && blockData && ((blockData.title_spans && blockData.title_spans.length > 0) || (blockData.title && blockData.title.length > 0)))
                width: parent.width
                spans: (blockData && blockData.title_spans && blockData.title_spans.length > 0) ? blockData.title_spans : ((blockData && blockData.title) ? [{ type: "text", value: blockData.title }] : undefined)
                color: isExample ? Theme.secondaryColor : Theme.highlightColor
                font.bold: true
                font.family: (typeof app !== "undefined" && app && app.docFontFamily && app.docFontFamily.length > 0) ? app.docFontFamily : Theme.fontFamily
                font.pixelSize: Math.round(Theme.fontSizeExtraSmall * ((typeof app !== "undefined" && app && app.fontScale) ? app.fontScale : 1.0))
                font.capitalization: Font.AllUppercase
                wrapMode: Text.WrapAtWordBoundaryOrAnywhere
            }

            Label {
                width: parent.width
                wrapMode: Text.WrapAtWordBoundaryOrAnywhere
                textFormat: Text.RichText
                text: BlockHtmlUtils.blocksToHtml((blockData && blockData.blocks) ? blockData.blocks : [], 0, "", { highlightColor: Theme.highlightColor, primaryColor: Theme.primaryColor, highlightBackgroundColor: Theme.highlightBackgroundColor }, (typeof bridge !== "undefined" && bridge) ? bridge.notes_dir : "", (typeof app !== "undefined" && app) ? app.allowExternalImages : true)
                font.family: (typeof app !== "undefined" && app && app.docFontFamily && app.docFontFamily.length > 0) ? app.docFontFamily : Theme.fontFamily
                font.pixelSize: Math.round((isOpen ? Theme.fontSizeMedium : Theme.fontSizeSmall) * ((typeof app !== "undefined" && app && app.fontScale) ? app.fontScale : 1.0))
                color: Theme.primaryColor
                linkColor: Theme.highlightColor
                onLinkActivated: function(link) {
                    if (link.indexOf("xref:") === 0) {
                        var target = link.substring(5)
                        if (target.indexOf(".adoc") === target.length - 5) {
                            target = target.substring(0, target.length - 5)
                        }
                        sidebarDelegate.xrefActivated(target)
                    } else if (link.indexOf("toggle:") === 0) {
                        var path = link.substring(7)
                        sidebarDelegate.checkboxToggled(sidebarDelegate.blockIndex, path)
                    } else if (link.indexOf("http") === 0) {
                        Qt.openUrlExternally(link)
                    }
                }
            }
        }
    }
}
