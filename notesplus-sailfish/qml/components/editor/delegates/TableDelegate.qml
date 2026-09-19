import QtQuick 2.6
import Sailfish.Silica 1.0
import "../"
import "../../common"
import "../../../js/BlockHtmlUtils.js" as BlockHtmlUtils

Column {
    id: tableDelegate
    property var blockData: ({})
    property string searchTerm: ""
    property int activeMatchIndexInBlock: -1
    signal xrefActivated(string target)

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

    Label {
        width: parent.width
        wrapMode: Text.WrapAtWordBoundaryOrAnywhere
        textFormat: Text.RichText
        text: {
            var html = (blockData && blockData.html) ? blockData.html : ""
            html = html.replace(/__LINK_COLOR__/g, Theme.highlightColor)
            if (searchTerm && searchTerm.length > 0) html = BlockHtmlUtils.highlightSearchTerms(html, searchTerm, activeMatchIndexInBlock)
            return html
        }
        font.family: app.resolvedFontFamily()
        font.pixelSize: app.scaledFontSize(Theme.fontSizeSmall)
        color: Theme.primaryColor
        linkColor: Theme.highlightColor
        onLinkActivated: function(link) {
            BlockHtmlUtils.handleLink(link, function(target) { tableDelegate.xrefActivated(target) }, null)
        }
    }
}
