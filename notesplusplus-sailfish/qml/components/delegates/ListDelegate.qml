import QtQuick 2.6
import Sailfish.Silica 1.0
import "../"
import "../../js/BlockHtmlUtils.js" as BlockHtmlUtils

Item {
    id: listDelegateItem
    property var blockData: ({})
    property var localBlockData: blockData
    property int blockIndex: -1
    property int renderCounter: 0

    signal xrefActivated(string target)
    signal checkboxToggled(int blockIndex, string itemPath)
    signal toggleLocalCheckbox(string path)

    height: loader.height
    anchors.left: parent ? parent.left : undefined
    anchors.right: parent ? parent.right : undefined

    Loader {
        id: loader
        anchors.left: parent.left
        anchors.right: parent.right
        sourceComponent: {
            if (!blockData || !blockData.type) return standardListComponent
            if (blockData.type === "description_list_item") return dlComponent
            if (blockData.type === "callout_list_item") return calloutComponent
            return standardListComponent
        }
    }

    Component {
        id: standardListComponent
        InlineText {
            text: (listDelegateItem.renderCounter >= 0) ? BlockHtmlUtils.blocksToHtml([localBlockData], (localBlockData && localBlockData.level) ? localBlockData.level : 0, "", { highlightColor: Theme.highlightColor, primaryColor: Theme.primaryColor, highlightBackgroundColor: Theme.highlightBackgroundColor }, (typeof bridge !== "undefined" && bridge) ? bridge.notes_dir : "", (typeof app !== "undefined" && app) ? app.allowExternalImages : true) : ""
            font.family: (typeof app !== "undefined" && app && app.docFontFamily && app.docFontFamily.length > 0) ? app.docFontFamily : Theme.fontFamily
            font.pixelSize: Math.round(Theme.fontSizeSmall * ((typeof app !== "undefined" && app && app.fontScale) ? app.fontScale : 1.0))
            anchors.left: parent ? parent.left : undefined
            anchors.right: parent ? parent.right : undefined
            anchors.leftMargin: Theme.horizontalPageMargin + ((localBlockData && localBlockData.level) ? (localBlockData.level * Theme.paddingLarge) : 0)
            anchors.rightMargin: Theme.horizontalPageMargin
            blockIndex: listDelegateItem.blockIndex
            onXrefActivated: function(target) { listDelegateItem.xrefActivated(target) }
            onCheckboxToggled: function(idx, itemPath) {
                listDelegateItem.toggleLocalCheckbox(itemPath)
                listDelegateItem.checkboxToggled(idx, itemPath)
            }
        }
    }

    Component {
        id: dlComponent
        Item {
            anchors.left: parent ? parent.left : undefined
            anchors.right: parent ? parent.right : undefined
            anchors.leftMargin: Theme.horizontalPageMargin
            anchors.rightMargin: Theme.horizontalPageMargin
            height: dlCol.height

            Column {
                id: dlCol
                anchors.left: parent.left
                anchors.right: parent.right
                spacing: Theme.paddingSmall / 2

                Label {
                    width: parent.width
                    textFormat: Text.RichText
                    text: blockData.term_spans ? ("<b>" + BlockHtmlUtils.spansToHtml(blockData.term_spans, { highlightColor: Theme.highlightColor, primaryColor: Theme.primaryColor, highlightBackgroundColor: Theme.highlightBackgroundColor }) + "</b>") : ("<b>" + BlockHtmlUtils.escapeHtml(blockData.term || "") + "</b>")
                    color: Theme.primaryColor
                    font.family: (typeof app !== "undefined" && app && app.docFontFamily && app.docFontFamily.length > 0) ? app.docFontFamily : Theme.fontFamily
                    font.pixelSize: Math.round(Theme.fontSizeMedium * ((typeof app !== "undefined" && app && app.fontScale) ? app.fontScale : 1.0))
                    wrapMode: Text.Wrap
                }

                Label {
                    width: parent.width - Theme.paddingMedium
                    x: Theme.paddingMedium
                    wrapMode: Text.WrapAtWordBoundaryOrAnywhere
                    textFormat: Text.RichText
                    text: BlockHtmlUtils.blocksToHtml(blockData.blocks || [], 0, "", { highlightColor: Theme.highlightColor, primaryColor: Theme.primaryColor, highlightBackgroundColor: Theme.highlightBackgroundColor })
                    font.family: (typeof app !== "undefined" && app && app.docFontFamily && app.docFontFamily.length > 0) ? app.docFontFamily : Theme.fontFamily
                    font.pixelSize: Math.round(Theme.fontSizeSmall * ((typeof app !== "undefined" && app && app.fontScale) ? app.fontScale : 1.0))
                    color: Theme.primaryColor
                    linkColor: Theme.highlightColor
                    onLinkActivated: function(link) {
                        if (link.indexOf("xref:") === 0) {
                            var target = link.substring(5)
                            if (target.indexOf(".adoc") === target.length - 5) {
                                target = target.substring(0, target.length - 5)
                            }
                            listDelegateItem.xrefActivated(target)
                        } else if (link.indexOf("toggle:") === 0) {
                            var path = link.substring(7)
                            listDelegateItem.checkboxToggled(listDelegateItem.blockIndex, path)
                        } else if (link.indexOf("http") === 0) {
                            Qt.openUrlExternally(link)
                        }
                    }
                }
            }
        }
    }

    Component {
        id: calloutComponent
        Item {
            anchors.left: parent ? parent.left : undefined
            anchors.right: parent ? parent.right : undefined
            anchors.leftMargin: Theme.horizontalPageMargin
            anchors.rightMargin: Theme.horizontalPageMargin
            height: Math.max(badgeLabel.height, calloutText.height)

            Label {
                id: badgeLabel
                text: "<" + (blockData.number || 1) + ">"
                color: Theme.highlightColor
                font.bold: true
                font.pixelSize: Math.round(Theme.fontSizeSmall * ((typeof app !== "undefined" && app && app.fontScale) ? app.fontScale : 1.0))
                anchors.left: parent.left
                anchors.top: parent.top
            }

            Label {
                id: calloutText
                anchors.left: badgeLabel.right
                anchors.leftMargin: Theme.paddingMedium
                anchors.right: parent.right
                wrapMode: Text.WrapAtWordBoundaryOrAnywhere
                textFormat: Text.RichText
                font.family: (typeof app !== "undefined" && app && app.docFontFamily && app.docFontFamily.length > 0) ? app.docFontFamily : Theme.fontFamily
                font.pixelSize: Math.round(Theme.fontSizeSmall * ((typeof app !== "undefined" && app && app.fontScale) ? app.fontScale : 1.0))
                color: Theme.primaryColor
                linkColor: Theme.highlightColor
                text: BlockHtmlUtils.blocksToHtml(blockData.blocks || [], 0, "", { highlightColor: Theme.highlightColor, primaryColor: Theme.primaryColor, highlightBackgroundColor: Theme.highlightBackgroundColor })
                onLinkActivated: function(link) {
                    if (link.indexOf("xref:") === 0) {
                        var target = link.substring(5)
                        if (target.indexOf(".adoc") === target.length - 5) {
                            target = target.substring(0, target.length - 5)
                        }
                        listDelegateItem.xrefActivated(target)
                    } else if (link.indexOf("toggle:") === 0) {
                        var path = link.substring(7)
                        listDelegateItem.checkboxToggled(listDelegateItem.blockIndex, path)
                    } else if (link.indexOf("http") === 0) {
                        Qt.openUrlExternally(link)
                    }
                }
            }
        }
    }
}
