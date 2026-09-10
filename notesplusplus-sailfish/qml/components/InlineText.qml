import QtQuick 2.6
import Sailfish.Silica 1.0
import "../js/BlockHtmlUtils.js" as BlockHtmlUtils

Label {
    id: inlineText
    property var spans: undefined
    property string searchTerm: ""
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
    text: (spans !== undefined && spans !== null) ? applySearchHighlight(renderSpans(spans)) : ""

    function applySearchHighlight(html) {
        if (!searchTerm || searchTerm.length === 0) return html
        return BlockHtmlUtils.highlightSearchTerms(html, searchTerm)
    }

    function renderSpans(spans) {
        if (!spans || spans.length === 0) return ""
        return BlockHtmlUtils.spansToHtml(spans, {
            highlightColor: Theme.highlightColor,
            primaryColor: Theme.primaryColor,
            highlightBackgroundColor: Theme.highlightBackgroundColor
        }, (typeof bridge !== "undefined" && bridge && bridge.notes_dir) ? bridge.notes_dir : "", (typeof app !== "undefined" && app && app.allowExternalImages !== undefined) ? app.allowExternalImages : true)
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
