import QtQuick 2.6
import Sailfish.Silica 1.0

Label {
    id: inlineText
    property var spans: []
    signal xrefActivated(string target)

    wrapMode: Text.Wrap
    textFormat: Text.RichText
    color: Theme.primaryColor
    text: renderSpans(spans)

    function renderSpans(spans) {
        if (!spans || spans.length === 0) return ""
        var html = ""
        for (var i = 0; i < spans.length; i++) {
            html += renderSpan(spans[i])
        }
        return html
    }

    function renderSpan(span) {
        if (!span) return ""
        switch (span.type) {
            case "text":
                return escapeHtml(span.value || "")
            case "bold":
                return "<b>" + renderSpans(span.spans) + "</b>"
            case "italic":
                return "<i>" + renderSpans(span.spans) + "</i>"
            case "code":
                return "<code style='background:" + Theme.highlightBackgroundColor + "'>" + escapeHtml(span.value || "") + "</code>"
            case "link":
                return "<a href='" + escapeHtml(span.url || "") + "' style='color:" + Theme.highlightColor + "'>" + escapeHtml(span.display || span.url || "") + "</a>"
            case "xref":
                return "<a href='xref:" + escapeHtml(span.target || "") + "' style='color:" + Theme.highlightColor + "'>" + escapeHtml(span.display || span.target || "") + "</a>"
            case "strikethrough":
                return "<s>" + renderSpans(span.spans) + "</s>"
            case "superscript":
                return "<sup>" + renderSpans(span.spans) + "</sup>"
            case "subscript":
                return "<sub>" + renderSpans(span.spans) + "</sub>"
            default:
                return escapeHtml(span.value || "")
        }
    }

    function escapeHtml(text) {
        return text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;")
    }

    onLinkActivated: function(link) {
        if (link.indexOf("xref:") === 0) {
            var target = link.substring(5)
            if (target.indexOf(".adoc") === target.length - 5) {
                target = target.substring(0, target.length - 5)
            }
            inlineText.xrefActivated(target)
        }
    }

}
