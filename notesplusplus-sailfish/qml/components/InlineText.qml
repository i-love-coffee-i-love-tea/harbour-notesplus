import QtQuick 2.6
import Sailfish.Silica 1.0
import "../js/BlockHtmlUtils.js" as BlockHtmlUtils

Label {
    id: inlineText
    property var spans: undefined
    property int blockIndex: -1
    signal xrefActivated(string target)
    signal checkboxToggled(int blockIndex, string itemPath)

    property bool pressed: false

    wrapMode: Text.WrapAtWordBoundaryOrAnywhere
    textFormat: Text.RichText
    color: Theme.primaryColor
    linkColor: Theme.highlightColor
    font.family: (typeof app !== "undefined" && app && app.docFontFamily && app.docFontFamily.length > 0) ? app.docFontFamily : Theme.fontFamily
    opacity: pressed ? 0.5 : 1.0
    text: (spans !== undefined && spans !== null) ? renderSpans(spans) : ""

    function renderSpans(spans) {
        if (!spans || spans.length === 0) return ""
        return BlockHtmlUtils.spansToHtml(spans, {
            highlightColor: Theme.highlightColor,
            primaryColor: Theme.primaryColor,
            highlightBackgroundColor: Theme.highlightBackgroundColor
        }, (typeof bridge !== "undefined" && bridge && bridge.notes_dir) ? bridge.notes_dir : "", (typeof app !== "undefined" && app && app.allowExternalImages !== undefined) ? app.allowExternalImages : true)
    }

    onLinkActivated: function(link) {
        if (link.indexOf("xref:") === 0) {
            var target = link.substring(5)
            if (target.indexOf(".adoc") === target.length - 5) {
                target = target.substring(0, target.length - 5)
            }
            inlineText.xrefActivated(target)
        } else if (link.indexOf("toggle:") === 0) {
            inlineText.pressed = true
            var path = link.substring(7)
            if (inlineText.blockIndex >= 0) {
                inlineText.checkboxToggled(inlineText.blockIndex, path)
            }
            feedbackTimer.restart()
        } else if (link.indexOf("http") === 0) {
            Qt.openUrlExternally(link)
        }
    }

    Timer {
        id: feedbackTimer
        interval: 300
        onTriggered: inlineText.pressed = false
    }
}
