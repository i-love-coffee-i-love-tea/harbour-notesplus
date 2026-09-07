import QtQuick 2.6
import Sailfish.Silica 1.0
import "../"
import "../../js/BlockHtmlUtils.js" as BlockHtmlUtils

Column {
    id: verseQuoteDelegate
    property var blockData: ({})
    property bool isVerse: blockData && blockData.type === "verse"
    signal xrefActivated(string target)

    anchors.left: parent ? parent.left : undefined
    anchors.right: parent ? parent.right : undefined
    anchors.leftMargin: Theme.horizontalPageMargin
    anchors.rightMargin: Theme.horizontalPageMargin
    spacing: Theme.paddingSmall / 2
    clip: true

    InlineText {
        visible: Boolean(blockData && ((blockData.title_spans && blockData.title_spans.length > 0) || (blockData.title && blockData.title.length > 0)))
        spans: (blockData && blockData.title_spans && blockData.title_spans.length > 0) ? blockData.title_spans : ((blockData && blockData.title) ? [{ type: "text", value: blockData.title }] : undefined)
        font.family: (typeof app !== "undefined" && app && app.docFontFamily && app.docFontFamily.length > 0) ? app.docFontFamily : Theme.fontFamily
        font.pixelSize: Math.round(Theme.fontSizeExtraSmall * ((typeof app !== "undefined" && app && app.fontScale) ? app.fontScale : 1.0))
        font.bold: true
        color: Theme.highlightColor
        wrapMode: Text.WrapAtWordBoundaryOrAnywhere
        width: parent.width
    }

    Rectangle {
        color: isVerse ? Theme.rgba(Theme.highlightBackgroundColor, 0.05) : "transparent"
        border.color: isVerse ? Theme.rgba(Theme.highlightColor, 0.4) : Theme.highlightColor
        border.width: isVerse ? 1 : 2
        radius: 4
        height: Math.max(innerCol.childrenRect.height, innerCol.implicitHeight) + Theme.paddingMedium * 2
        anchors.left: parent.left
        anchors.right: parent.right

        Rectangle {
            visible: isVerse
            anchors.left: parent.left
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            width: 3
            color: Theme.highlightColor
            radius: 2
        }

        Column {
            id: innerCol
            anchors {
                left: parent.left
                right: parent.right
                top: parent.top
                leftMargin: isVerse ? (Theme.paddingMedium + 4) : Theme.paddingMedium
                rightMargin: Theme.paddingMedium
                topMargin: Theme.paddingMedium
            }
            spacing: Theme.paddingSmall / 3

            Repeater {
                visible: isVerse
                model: isVerse ? ((blockData && blockData.lines_spans && blockData.lines_spans.length > 0) ? blockData.lines_spans : ((blockData && blockData.lines) ? blockData.lines : [])) : []
                delegate: Label {
                    width: innerCol.width
                    wrapMode: Text.WrapAtWordBoundaryOrAnywhere
                    textFormat: Text.RichText
                    text: {
                        if (modelData && typeof modelData !== "string") {
                            return BlockHtmlUtils.spansToHtml(modelData, { highlightColor: Theme.highlightColor, primaryColor: Theme.primaryColor, highlightBackgroundColor: Theme.highlightBackgroundColor }, (typeof bridge !== "undefined" && bridge && bridge.notes_dir) ? bridge.notes_dir : "", (typeof app !== "undefined" && app && app.allowExternalImages !== undefined) ? app.allowExternalImages : true)
                        } else if (typeof modelData === "string") {
                            return BlockHtmlUtils.escapeHtml(modelData)
                        }
                        return ""
                    }
                    font.italic: true
                    font.family: (typeof app !== "undefined" && app && app.docFontFamily && app.docFontFamily.length > 0) ? app.docFontFamily : Theme.fontFamily
                    font.pixelSize: Math.round(Theme.fontSizeMedium * ((typeof app !== "undefined" && app && app.fontScale) ? app.fontScale : 1.0))
                    color: Theme.primaryColor
                    linkColor: Theme.highlightColor
                    onLinkActivated: function(link) {
                        if (link.indexOf("xref:") === 0) {
                            var target = link.substring(5)
                            if (target.indexOf(".adoc") === target.length - 5) {
                                target = target.substring(0, target.length - 5)
                            }
                            verseQuoteDelegate.xrefActivated(target)
                        } else if (link.indexOf("http") === 0) {
                            Qt.openUrlExternally(link)
                        }
                    }
                }
            }

            Label {
                visible: !isVerse
                width: parent.width
                wrapMode: Text.WrapAtWordBoundaryOrAnywhere
                textFormat: Text.RichText
                text: (!isVerse && blockData && blockData.blocks && blockData.blocks.length > 0)
                      ? BlockHtmlUtils.blocksToHtml(blockData.blocks, 0, "", { highlightColor: Theme.highlightColor, primaryColor: Theme.primaryColor, highlightBackgroundColor: Theme.highlightBackgroundColor }, (typeof bridge !== "undefined" && bridge && bridge.notes_dir) ? bridge.notes_dir : "", (typeof app !== "undefined" && app && app.allowExternalImages !== undefined) ? app.allowExternalImages : true)
                      : ("<i>" + BlockHtmlUtils.escapeHtml((blockData && (blockData.raw || "")) || "") + "</i>")
                font.italic: true
                font.family: (typeof app !== "undefined" && app && app.docFontFamily && app.docFontFamily.length > 0) ? app.docFontFamily : Theme.fontFamily
                font.pixelSize: Math.round(Theme.fontSizeMedium * ((typeof app !== "undefined" && app && app.fontScale) ? app.fontScale : 1.0))
                color: Theme.primaryColor
                linkColor: Theme.highlightColor
                onLinkActivated: function(link) {
                    if (link.indexOf("xref:") === 0) {
                        var target = link.substring(5)
                        if (target.indexOf(".adoc") === target.length - 5) {
                            target = target.substring(0, target.length - 5)
                        }
                        verseQuoteDelegate.xrefActivated(target)
                    } else if (link.indexOf("http") === 0) {
                        Qt.openUrlExternally(link)
                    }
                }
            }

            Label {
                visible: Boolean(blockData && (blockData.attribution || blockData.citation))
                width: parent.width
                horizontalAlignment: Text.AlignRight
                text: {
                    var attr = (blockData && blockData.attribution) ? blockData.attribution : ""
                    var cit = (blockData && blockData.citation) ? blockData.citation : ""
                    if (attr && cit) return "\u2014 " + attr + ", " + cit
                    if (attr) return "\u2014 " + attr
                    if (cit) return "\u2014 " + cit
                    return ""
                }
                font.family: (typeof app !== "undefined" && app && app.docFontFamily && app.docFontFamily.length > 0) ? app.docFontFamily : Theme.fontFamily
                font.pixelSize: Math.round(Theme.fontSizeSmall * ((typeof app !== "undefined" && app && app.fontScale) ? app.fontScale : 1.0))
                font.italic: true
                color: Theme.highlightColor
            }
        }
    }
}
