import QtQuick 2.6
import Sailfish.Silica 1.0
import "../"
import "../../js/BlockHtmlUtils.js" as BlockHtmlUtils

Column {
    id: verseQuoteDelegate
    property var blockData: ({})
    property int blockIndex: -1
    property string searchTerm: ""
    property bool isVerse: blockData && blockData.type === "verse"
    signal xrefActivated(string target)
    signal checkboxToggled(int blockIndex, string itemPath)

    anchors.left: parent ? parent.left : undefined
    anchors.right: parent ? parent.right : undefined
    anchors.leftMargin: Theme.horizontalPageMargin
    anchors.rightMargin: Theme.horizontalPageMargin
    spacing: Theme.paddingSmall / 2
    clip: true

    InlineText {
        visible: Boolean(blockData && ((blockData.title_spans && blockData.title_spans.length > 0) || (blockData.title && blockData.title.length > 0)))
        preRenderedHtml: (blockData && blockData.html) ? blockData.html.replace(/__LINK_COLOR__/g, Theme.highlightColor) : ""
        font.family: app.resolvedFontFamily()
        font.pixelSize: app.scaledFontSize(Theme.fontSizeExtraSmall)
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

            Label {
                visible: !isVerse
                width: parent.width
                wrapMode: Text.WrapAtWordBoundaryOrAnywhere
                textFormat: Text.RichText
                text: {
                    // Use Rust HTML for blockquote body, strip outer blockquote wrapper (QML provides border)
                    var html = (blockData && blockData.html) ? blockData.html : ""
                    html = html.replace(/<blockquote[^>]*>/, "").replace(/<\/blockquote>$/, "")
                    html = html.replace(/__LINK_COLOR__/g, Theme.highlightColor)
                    if (searchTerm && searchTerm.length > 0) html = BlockHtmlUtils.highlightSearchTerms(html, searchTerm)
                    return html || ("<i>" + ((blockData && (blockData.raw || "")) || "") + "</i>")
                }
                font.italic: true
                font.family: app.resolvedFontFamily()
                font.pixelSize: app.scaledFontSize(Theme.fontSizeMedium)
                color: Theme.primaryColor
                linkColor: Theme.highlightColor
                onLinkActivated: function(link) {
                    BlockHtmlUtils.handleLink(link, function(target) { verseQuoteDelegate.xrefActivated(target) }, function(path) { verseQuoteDelegate.checkboxToggled(verseQuoteDelegate.blockIndex, path) })
                }
            }

            Label {
                visible: isVerse
                width: parent.width
                wrapMode: Text.WrapAtWordBoundaryOrAnywhere
                textFormat: Text.RichText
                text: {
                    // Use Rust HTML for verse body, strip outer blockquote wrapper
                    var html = (blockData && blockData.html) ? blockData.html : ""
                    html = html.replace(/<blockquote[^>]*>/, "").replace(/<\/blockquote>$/, "")
                    html = html.replace(/__LINK_COLOR__/g, Theme.highlightColor)
                    if (searchTerm && searchTerm.length > 0) html = BlockHtmlUtils.highlightSearchTerms(html, searchTerm)
                    return html
                }
                font.italic: true
                font.family: app.resolvedFontFamily()
                font.pixelSize: app.scaledFontSize(Theme.fontSizeMedium)
                color: Theme.primaryColor
                linkColor: Theme.highlightColor
                onLinkActivated: function(link) {
                    BlockHtmlUtils.handleLink(link, function(target) { verseQuoteDelegate.xrefActivated(target) }, function(path) { verseQuoteDelegate.checkboxToggled(verseQuoteDelegate.blockIndex, path) })
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
                font.family: app.resolvedFontFamily()
                font.pixelSize: app.scaledFontSize(Theme.fontSizeSmall)
                font.italic: true
                color: Theme.highlightColor
            }
        }
    }
}
