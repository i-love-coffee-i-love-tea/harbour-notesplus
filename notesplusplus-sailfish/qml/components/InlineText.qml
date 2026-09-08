import QtQuick 2.6
import Sailfish.Silica 1.0

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
    Behavior on opacity { FadeAnimator {} }
    text: (spans !== undefined && spans !== null) ? renderSpans(spans) : ""

    onSpansChanged: {
        if (spans !== undefined && spans !== null) {
            text = renderSpans(spans)
        }
    }

    Component.onCompleted: {
        if (spans !== undefined && spans !== null) {
            text = renderSpans(spans)
        }
    }

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
        if (typeof span === "string") return escapeHtml(span)
        switch (span.type) {
            case "text":
                return escapeHtml(span.value || "")
            case "bold":
                return "<b>" + renderSpans(span.spans) + "</b>"
            case "italic":
                return "<i>" + renderSpans(span.spans) + "</i>"
            case "code":
                return "<code style='background:#18181c;color:#f2f2f7;padding:1px 4px;border-radius:0;font-family:monospace;word-break:break-all;'>" + (span.spans ? renderSpans(span.spans) : escapeHtml(span.value || "")) + "</code>"
            case "quote":
                return "&ldquo;" + renderSpans(span.spans) + "&rdquo;"
            case "squote":
                return "&lsquo;" + renderSpans(span.spans) + "&rsquo;"
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
            case "image":
                var wAttr = span.width ? (" width='" + escapeHtml(span.width) + "'") : " style='max-width:100%;'"
                var src = resolveImagePath(span.target || "")
                return "<img src='" + escapeHtml(src) + "'" + wAttr + " alt='" + escapeHtml(span.alt || "") + "' />"
            case "icon":
                return renderIcon(span.name || "", span.options || "")
            case "footnote":
                return "<sup style='color:" + Theme.highlightColor + "'>[" + escapeHtml(span.text || span.id || "") + "]</sup>"
            case "callout":
                return "<b style='color:" + Theme.highlightColor + ";background:" + Theme.rgba(Theme.highlightBackgroundColor, 0.25) + ";padding:1px 4px;border-radius:8px;'>&lt;" + (span.number || 1) + "&gt;</b>"
            case "mark":
                return "<mark style='background:" + Theme.rgba(Theme.highlightColor, 0.25) + ";color:" + Theme.primaryColor + ";padding:1px 3px;border-radius:2px;'>" + renderSpans(span.spans || []) + "</mark>"
            case "kbd":
                return "<kbd style='background:#2a2a2e;color:#f2f2f7;padding:1px 5px;border:1px solid #444;border-radius:4px;font-family:monospace;'>" + escapeHtml((span.keys || []).join("+")) + "</kbd>"
            case "btn":
                return "<span style='background:" + Theme.rgba(Theme.highlightColor, 0.2) + ";color:" + Theme.highlightColor + ";padding:1px 6px;border:1px solid " + Theme.highlightColor + ";border-radius:4px;font-weight:bold;'>[" + escapeHtml(span.text || "") + "]</span>"
            case "menu":
                return "<b style='color:" + Theme.highlightColor + ";'>" + (span.items ? span.items.map(escapeHtml).join(" \u25B8 ") : "") + "</b>"
            case "pass":
                return span.value || ""
            default:
                return escapeHtml(span.value || "")
        }
    }

    function resolveImagePath(target) {
        if (!target) return ""
        if (target.indexOf("http://") === 0 || target.indexOf("https://") === 0) {
            var allowExt = (typeof app !== "undefined" && app && app.allowExternalImages !== undefined) ? app.allowExternalImages : true
            return allowExt ? target : ""
        }
        if (target.indexOf("file://") === 0 || target.indexOf("/") === 0) {
            return target
        }
        var notesDir = (typeof bridge !== "undefined" && bridge && bridge.notes_dir) ? bridge.notes_dir : ""
        if (notesDir.length > 0) {
            return "file://" + notesDir + "/" + target
        }
        return "file:///usr/share/harbour-notesplusplus/examples/" + target
    }

    function renderIcon(iconName, options) {
        if (!iconName) return ""
        var name = iconName.toLowerCase()
        if (name === "heart") {
            return "<span style='color:" + Theme.highlightColor + ";font-size:" + Theme.fontSizeMedium + "px;'>&#10084;</span>"
        } else if (name === "star") {
            return "<span style='color:" + Theme.highlightColor + ";font-size:" + Theme.fontSizeMedium + "px;'>&#9733;</span>"
        } else if (name === "check" || name === "check-circle") {
            return "<span style='color:" + Theme.highlightColor + ";font-weight:bold;'>&#10004;</span>"
        } else if (name === "info" || name === "info-circle") {
            return "<span style='color:" + Theme.highlightColor + ";font-weight:bold;'>&#8505;</span>"
        } else if (name === "warning" || name === "exclamation" || name === "alert") {
            return "<span style='color:" + Theme.highlightColor + ";font-weight:bold;'>&#9888;</span>"
        } else if (name === "folder") {
            return "<span style='color:" + Theme.highlightColor + ";'>&#128193;</span>"
        } else if (name === "file" || name === "document") {
            return "<span style='color:" + Theme.highlightColor + ";'>&#128196;</span>"
        } else if (name === "tag" || name === "tags") {
            return "<span style='color:" + Theme.highlightColor + ";'>&#127991;</span>"
        } else if (name === "search") {
            return "<span style='color:" + Theme.highlightColor + ";'>&#128269;</span>"
        } else if (name === "gear" || name === "cog" || name === "settings") {
            return "<span style='color:" + Theme.highlightColor + ";'>&#9881;</span>"
        }
        return "<span style='color:" + Theme.highlightColor + ";'>:" + escapeHtml(iconName) + ":</span>"
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
