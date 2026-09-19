import QtQuick 2.6
import Sailfish.Silica 1.0
import "../../js/BlockHtmlUtils.js" as BlockHtmlUtils

Label {
    id: inlineText
    property var spans: undefined
    property string preRenderedHtml: ""
    property string searchTerm: ""
    property int activeMatchIndexInBlock: -1
    property int blockIndex: -1
    signal xrefActivated(string target)
    signal checkboxToggled(int blockIndex, string itemPath)

    property bool pressed: false

    wrapMode: Text.WrapAtWordBoundaryOrAnywhere
    textFormat: Text.RichText
    color: Theme.primaryColor
    linkColor: Theme.highlightColor
    font.family: app.resolvedFontFamily()
    opacity: pressed ? 0.5 : 1.0
    text: {
        if (preRenderedHtml && preRenderedHtml.length > 0) {
            return applySearchHighlight(substituteThemeColors(preRenderedHtml))
        }
        return ""
    }

    function substituteThemeColors(html) {
        // Replace placeholders from Rust renderer with actual QML theme colors
        return html.replace(/__LINK_COLOR__/g, Theme.highlightColor)
    }

    function applySearchHighlight(html) {
        if (!searchTerm || searchTerm.length === 0) return html
        return BlockHtmlUtils.highlightSearchTerms(html, searchTerm, activeMatchIndexInBlock)
    }

    onLinkActivated: function(link) {
        BlockHtmlUtils.handleLink(link, function(target) { inlineText.xrefActivated(target) }, function(path) {
            inlineText.pressed = true
            if (inlineText.blockIndex >= 0) {
                inlineText.checkboxToggled(inlineText.blockIndex, path)
            }
            feedbackTimer.restart()
        })
    }

    Timer {
        id: feedbackTimer
        interval: 300
        onTriggered: inlineText.pressed = false
    }
}
