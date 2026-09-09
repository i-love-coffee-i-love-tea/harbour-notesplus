.pragma library

function escapeHtml(text) {
    if (!text) return ""
    var s = "" + text
    if (s.indexOf("&") === -1 && s.indexOf("<") === -1 && s.indexOf(">") === -1) {
        return s
    }
    return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;")
}

function spansToPlainText(spans) {
    if (!spans || spans.length === 0) return ""
    if (spans.length === 1 && spans[0]) {
        var first = spans[0]
        if (first.value !== undefined) return first.value
        if (first.display !== undefined) return first.display
    }
    var txt = ""
    for (var i = 0; i < spans.length; i++) {
        var s = spans[i]
        if (!s) continue
        if (s.value !== undefined) txt += s.value
        else if (s.display !== undefined) txt += s.display
        else if (s.spans) txt += spansToPlainText(s.spans)
    }
    return txt
}

function resolveImagePath(target, notesDir, allowExternal) {
    if (!target) return ""
    if (target.indexOf("http://") === 0 || target.indexOf("https://") === 0) {
        return (allowExternal !== false) ? target : ""
    }
    if (target.indexOf("file://") === 0 || target.indexOf("/") === 0) {
        return target
    }
    if (notesDir && notesDir.length > 0) {
        return "file://" + notesDir + "/" + target
    }
    return "file:///usr/share/harbour-notesplusplus/examples/" + target
}

function renderIcon(iconName, options, highlightColor) {
    if (!iconName) return ""
    var name = iconName.toLowerCase()
    var color = highlightColor || "#ffffff"
    if (name === "heart") {
        return "<span style='color:" + color + ";'>&#10084;</span>"
    } else if (name === "star") {
        return "<span style='color:" + color + ";'>&#9733;</span>"
    } else if (name === "check" || name === "check-circle") {
        return "<span style='color:" + color + ";font-weight:bold;'>&#10004;</span>"
    } else if (name === "info" || name === "info-circle") {
        return "<span style='color:" + color + ";font-weight:bold;'>&#8505;</span>"
    } else if (name === "warning" || name === "exclamation" || name === "alert") {
        return "<span style='color:" + color + ";font-weight:bold;'>&#9888;</span>"
    } else if (name === "folder") {
        return "<span style='color:" + color + ";'>&#128193;</span>"
    } else if (name === "file" || name === "document") {
        return "<span style='color:" + color + ";'>&#128196;</span>"
    } else if (name === "tag" || name === "tags") {
        return "<span style='color:" + color + ";'>&#127991;</span>"
    } else if (name === "search") {
        return "<span style='color:" + color + ";'>&#128269;</span>"
    } else if (name === "gear" || name === "cog" || name === "settings") {
        return "<span style='color:" + color + ";'>&#9881;</span>"
    }
    return "<span style='color:" + color + ";'>:" + escapeHtml(iconName) + ":</span>"
}

function spanToHtml(span, themeColors, notesDir, allowExternal) {
    if (!span) return ""
    var highlight = (themeColors && themeColors.highlightColor) ? themeColors.highlightColor : "#0088cc"
    var primary = (themeColors && themeColors.primaryColor) ? themeColors.primaryColor : "#ffffff"
    var highlightBg = (themeColors && themeColors.highlightBackgroundColor) ? themeColors.highlightBackgroundColor : "rgba(0,136,204,0.25)"

    switch (span.type) {
        case "text": return escapeHtml(span.value || "")
        case "bold": return "<b>" + spansToHtml(span.spans || [], themeColors, notesDir, allowExternal) + "</b>"
        case "italic": return "<i>" + spansToHtml(span.spans || [], themeColors, notesDir, allowExternal) + "</i>"
        case "code": return "<code style='background:#18181c;color:#f2f2f7;padding:1px 4px;border-radius:0;font-family:monospace;word-break:break-all;'>" + (span.spans ? spansToHtml(span.spans, themeColors, notesDir, allowExternal) : escapeHtml(span.value || "")) + "</code>"
        case "quote": return "&ldquo;" + spansToHtml(span.spans || [], themeColors, notesDir, allowExternal) + "&rdquo;"
        case "squote": return "&lsquo;" + spansToHtml(span.spans || [], themeColors, notesDir, allowExternal) + "&rsquo;"
        case "link": return "<a href='" + escapeHtml(span.url || "") + "' style='color:" + highlight + "'>" + escapeHtml(span.display || span.url || "") + "</a>"
        case "xref": return "<a href='xref:" + escapeHtml(span.target || "") + "' style='color:" + highlight + "'>" + escapeHtml(span.display || span.target || "") + "</a>"
        case "strikethrough": return "<s>" + spansToHtml(span.spans || [], themeColors, notesDir, allowExternal) + "</s>"
        case "superscript": return "<sup>" + spansToHtml(span.spans || [], themeColors, notesDir, allowExternal) + "</sup>"
        case "subscript": return "<sub>" + spansToHtml(span.spans || [], themeColors, notesDir, allowExternal) + "</sub>"
        case "image":
            var wAttr = span.width ? (" width='" + escapeHtml(span.width) + "'") : " style='max-width:100%;'"
            var src = resolveImagePath(span.target || "", notesDir, allowExternal)
            return "<img src='" + escapeHtml(src) + "'" + wAttr + " alt='" + escapeHtml(span.alt || "") + "' />"
        case "icon":
            return renderIcon(span.name || "", span.options || "", highlight)
        case "footnote":
            return "<sup style='color:" + highlight + "'>[" + escapeHtml(span.text || span.id || "") + "]</sup>"
        case "callout":
            return "<b style='color:" + highlight + ";background:" + highlightBg + ";padding:1px 4px;border-radius:8px;'>&lt;" + (span.number || 1) + "&gt;</b>"
        case "mark":
            return "<mark style='background:" + highlightBg + ";color:" + primary + ";padding:1px 3px;border-radius:2px;'>" + spansToHtml(span.spans || [], themeColors, notesDir, allowExternal) + "</mark>"
        case "kbd":
            return "<kbd style='background:#2a2a2e;color:#f2f2f7;padding:1px 5px;border:1px solid #444;border-radius:4px;font-family:monospace;'>" + escapeHtml((span.keys || []).join("+")) + "</kbd>"
        case "btn":
            return "<span style='background:" + highlightBg + ";color:" + highlight + ";padding:1px 6px;border:1px solid " + highlight + ";border-radius:4px;font-weight:bold;'>[" + escapeHtml(span.text || "") + "]</span>"
        case "menu":
            return "<b style='color:" + highlight + ";'>" + (span.items ? span.items.map(escapeHtml).join(" \u25B8 ") : "") + "</b>"
        case "pass":
            return span.value || ""
        default: return escapeHtml(span.value || "")
    }
}

function spansToHtml(spans, themeColors, notesDir, allowExternal) {
    if (!spans || spans.length === 0) return ""

    if (spans.length === 1 && spans[0]) {
        var first = spans[0]
        if (first.type === "text" && first.value !== undefined) {
            return escapeHtml(first.value)
        }
    }

    var html = ""
    for (var i = 0; i < spans.length; i++) {
        html += spanToHtml(spans[i], themeColors, notesDir, allowExternal)
    }
    return html
}

function blocksToHtml(blocks, depth, pathPrefix, themeColors, notesDir, allowExternal) {
    if (!blocks) return ""
    if (depth === undefined) depth = 0
    if (pathPrefix === undefined) pathPrefix = ""

    var items = []
    for (var i = 0; i < blocks.length; i++) {
        items.push({ block: blocks[i], path: pathPrefix })
    }
    return blocksToHtmlFromItems(items, depth, themeColors, notesDir, allowExternal)
}

function blocksToHtmlFromItems(items, depth, themeColors, notesDir, allowExternal) {
    if (!items || items.length === 0) return ""
    if (depth === undefined) depth = 0
    var primary = (themeColors && themeColors.primaryColor) ? themeColors.primaryColor : "#ffffff"
    var highlight = (themeColors && themeColors.highlightColor) ? themeColors.highlightColor : "#0088cc"
    var highlightBg = (themeColors && themeColors.highlightBackgroundColor) ? themeColors.highlightBackgroundColor : "rgba(0,136,204,0.25)"
    var html = ""
    var listBuf = []
    var listType = ""

    function flushList() {
        if (listBuf.length === 0) return
        for (var i = 0; i < listBuf.length; i++) {
            var item = listBuf[i]
            var b = item.block
            var itemPath = item.path
            if (!b) continue
            var itemText = ""
            if (b.blocks && b.blocks.length > 0 && b.blocks[0].spans) {
                itemText = spansToHtml(b.blocks[0].spans, themeColors, notesDir, allowExternal)
            } else if (b.spans) {
                itemText = spansToHtml(b.spans, themeColors, notesDir, allowExternal)
            } else if (b.raw_text || b.raw) {
                itemText = escapeHtml(b.raw_text || b.raw)
            }
            var isChecked = b.checked
            var checkMark = ""
            if (isChecked === true) checkMark = "\u2611"
            else if (isChecked === false) checkMark = "\u2610"

            var markerHtml = ""
            var markerColor = primary
            var markerWidth = "18px"

            if (checkMark) {
                markerHtml = "<a href='toggle:" + itemPath + "' style='color:" + primary + ";text-decoration:none'>" + checkMark + "</a>"
                markerWidth = "24px"
                if (itemText.indexOf("<a ") === -1) {
                    itemText = "<a href='toggle:" + itemPath + "' style='color:" + primary + ";text-decoration:none'>" + itemText + "</a>"
                }
            } else if (listType === "callout") {
                markerHtml = "&lt;" + (b.number || (i + 1)) + "&gt;"
                markerColor = highlight
                markerWidth = "28px"
            } else if (listType === "ordered") {
                var m = (b.marker && b.marker.length > 0) ? b.marker : ((i + 1) + ".")
                markerHtml = escapeHtml(m)
                markerWidth = "28px"
            } else {
                var bulletChars = ["&bull;", "&#9702;", "&#9642;", "&#8212;", "&bull;"]
                var lvl = (b.level !== undefined) ? b.level : depth
                markerHtml = bulletChars[lvl % bulletChars.length]
                markerWidth = "18px"
            }

            var childHtml = ""
            if (b.blocks && b.blocks.length > 1) {
                var childList = []
                for (var k = 1; k < b.blocks.length; k++) {
                    var childPath = itemPath ? (itemPath + "." + k) : ("" + k)
                    childList.push({ block: b.blocks[k], path: childPath })
                }
                childHtml = blocksToHtmlFromItems(childList, depth + 1, themeColors, notesDir, allowExternal)
            }

            var indentPad = (lvl > 0) ? (lvl * 20) : ((depth > 0) ? (depth * 20) : 0)
            html += "<table width='100%' style='width:100%;border-collapse:collapse;margin:2px 0;'>"
            html += "<tr>"
            if (indentPad > 0) {
                html += "<td width='" + indentPad + "px' style='width:" + indentPad + "px;padding:0;'></td>"
            }
            html += "<td width='" + markerWidth + "' style='width:" + markerWidth + ";vertical-align:top;padding-right:6px;font-weight:" + (checkMark ? "normal" : "bold") + ";color:" + markerColor + ";'>" + markerHtml + "</td>"
            html += "<td width='99%' style='vertical-align:top;word-wrap:break-word;'>" + itemText + childHtml + "</td>"
            html += "</tr>"
            html += "</table>"
        }
        listBuf = []
        listType = ""
    }

    for (var i = 0; i < items.length; i++) {
        var it = items[i]
        var b = it.block
        if (!b) continue
        var t = b.type
        if (t === "unordered_list_item" || t === "ordered_list_item" || t === "callout_list_item") {
            var curType = (t === "unordered_list_item") ? "unordered" : ((t === "ordered_list_item") ? "ordered" : "callout")
            if (listBuf.length > 0 && curType !== listType) {
                flushList()
            }
            listType = curType
            listBuf.push(it)
        } else {
            flushList()
            if (t === "paragraph") {
                html += "<p style='margin:4px 0;'>" + spansToHtml(b.spans, themeColors, notesDir, allowExternal) + "</p>"
            } else if (t === "heading") {
                html += "<h3 style='color:" + highlight + ";margin:6px 0 2px 0;'>" + spansToHtml(b.spans, themeColors, notesDir, allowExternal) + "</h3>"
            } else if (t === "code_block" || t === "literal_block") {
                var codeLines = (b.lines || []).map(function(l) { return escapeHtml(l) }).join("<br/>")
                html += "<pre style='background:#18181c;color:#f2f2f7;padding:6px;border-radius:0;font-family:monospace;margin:4px 0;word-break:break-all;'>" + codeLines + "</pre>"
            } else if (t === "description_list_item") {
                var termTxt = b.term_spans ? spansToHtml(b.term_spans, themeColors, notesDir, allowExternal) : escapeHtml(b.term || "")
                var descHtml = ""
                if (b.blocks && b.blocks.length > 0 && b.blocks[0].spans) {
                    descHtml = spansToHtml(b.blocks[0].spans, themeColors, notesDir, allowExternal)
                    if (b.blocks.length > 1) {
                        var restBlocks = []
                        for (var rb = 1; rb < b.blocks.length; rb++) {
                            restBlocks.push(b.blocks[rb])
                        }
                        descHtml += "<br/>" + blocksToHtml(restBlocks, depth + 1, "", themeColors, notesDir, allowExternal)
                    }
                } else {
                    descHtml = blocksToHtml(b.blocks || [], depth, "", themeColors, notesDir, allowExternal)
                }
                html += "<p style='margin:4px 0;'><b>" + termTxt + ":</b> " + descHtml + "</p>"
            } else if (t === "blockquote" || t === "verse") {
                var quoteBody = b.blocks ? blocksToHtml(b.blocks, depth, "", themeColors, notesDir, allowExternal) : (b.lines ? (b.lines || []).map(function(l) { return escapeHtml(l) }).join("<br/>") : "")
                html += "<blockquote style='margin:4px 0;padding-left:8px;border-left:2px solid " + highlight + ";color:" + primary + ";'>" + quoteBody + "</blockquote>"
            } else if (t === "admonition") {
                html += "<div style='margin:4px 0;padding:6px;border-left:3px solid " + highlight + ";background:" + highlightBg + ";'><b>" + escapeHtml(b.kind || "NOTE") + ":</b> " + blocksToHtml(b.blocks || [], depth, "", themeColors, notesDir, allowExternal) + "</div>"
            } else if (t === "sidebar" || t === "example" || t === "open") {
                html += "<div style='margin:4px 0;padding:6px;border:1px solid " + highlight + ";border-radius:0;'>" + (b.title ? ("<b style='color:" + highlight + ";'>" + escapeHtml(b.title) + "</b><br/>") : "") + blocksToHtml(b.blocks || [], depth, "", themeColors, notesDir, allowExternal) + "</div>"
            } else if (t === "image") {
                var imgW = b.width ? (" width='" + escapeHtml(b.width) + "'") : " style='max-width:100%;'"
                html += "<img src='" + escapeHtml(resolveImagePath(b.target || "", notesDir, allowExternal)) + "'" + imgW + " alt='" + escapeHtml(b.alt || "") + "' />"
            } else if (b.spans) {
                html += "<p style='margin:4px 0;'>" + spansToHtml(b.spans, themeColors, notesDir, allowExternal) + "</p>"
            } else if (b.raw || b.raw_text) {
                html += "<p style='margin:4px 0;'>" + escapeHtml(b.raw || b.raw_text || "") + "</p>"
            } else if (t === "empty_line") {
                html += "<div style='height:8px;'></div>"
            }
        }
    }
    flushList()
    return html
}
