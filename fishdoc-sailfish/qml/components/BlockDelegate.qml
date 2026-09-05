import QtQuick 2.6
import Sailfish.Silica 1.0

Item {
    id: delegate
    property var blockData: ({})
    property var allBlocks: []
    property int blockIndex: -1
    property var localBlockData: blockData ? JSON.parse(JSON.stringify(blockData)) : ({})
    property int renderCounter: 0
    property bool isTocCollapsed: false
    property string searchTerm: ""
    property bool isSearchMatch: {
        if (!searchTerm || searchTerm.length === 0 || !blockData) return false
        var q = searchTerm.toLowerCase().trim()
        var textToSearch = ""
        if (blockData.raw_text) textToSearch += " " + blockData.raw_text
        if (blockData.raw) textToSearch += " " + blockData.raw
        if (blockData.lines) textToSearch += " " + blockData.lines.join(" ")
        if (blockData.term) textToSearch += " " + blockData.term
        if (blockData.title) textToSearch += " " + blockData.title
        return textToSearch.toLowerCase().indexOf(q) !== -1
    }

    Rectangle {
        anchors.left: parent.left
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        width: 4
        color: Theme.highlightColor
        visible: delegate.isSearchMatch
        z: 2
    }

    onBlockDataChanged: {
        localBlockData = blockData ? JSON.parse(JSON.stringify(blockData)) : ({})
        renderCounter++
    }

    signal xrefActivated(string target)
    signal checkboxToggled(int blockIndex, string itemPath)
    signal toggleToc(int blockIndex)
    signal editRequested(int blockIndex, string rawText)
    signal jumpToBlock(int targetIndex)

    function toggleLocalCheckbox(path) {
        if (!localBlockData) return
        var target = localBlockData
        if (path && path.length > 0) {
            var parts = path.split(".")
            for (var p = 0; p < parts.length; p++) {
                var idx = parseInt(parts[p], 10)
                if (target.blocks && target.blocks[idx]) {
                    target = target.blocks[idx]
                } else {
                    return
                }
            }
        }
        if (target.checked !== undefined && target.checked !== null) {
            target.checked = !target.checked
            renderCounter++
        }
    }

    height: contentItem.height + Theme.paddingSmall

    MouseArea {
        anchors.fill: parent
        onClicked: {
            var raw = delegate.blockData.raw_text || delegate.blockData.raw || ""
            delegate.editRequested(delegate.blockIndex, raw)
        }
    }

    Loader {
        id: contentItem
        z: 1
        width: parent.width
        sourceComponent: {
            if (!blockData || !blockData.type) return emptyComponent
            switch (blockData.type) {
                case "heading": return headingComponent
                case "paragraph": return paragraphComponent
                case "ordered_list_item": return orderedListComponent
                case "unordered_list_item": return unorderedListComponent
                case "code_block": return codeBlockComponent
                case "literal_block": return literalBlockComponent
                case "blockquote": return blockquoteComponent
                case "table": return tableComponent
                case "image": return imageComponent
                case "sidebar": return sidebarComponent
                case "example": return exampleComponent
                case "open": return openComponent
                case "page_break": return pageBreakComponent
                case "description_list_item": return descriptionListComponent
                case "callout_list_item": return calloutListComponent
                case "admonition": return admonitionComponent
                case "comment": return commentComponent
                case "toc": return tocComponent
                case "horizontal_rule": return hrComponent
                case "empty_line": return emptyComponent
                default: return paragraphComponent
            }
        }
    }

    Component {
        id: commentComponent
        Label {
            text: "// " + (blockData ? (blockData.text || "") : "")
            x: Theme.horizontalPageMargin
            width: parent.width - Theme.horizontalPageMargin * 2
            font.italic: true
            font.family: "monospace"
            font.pixelSize: Math.round(Theme.fontSizeSmall * ((typeof app !== "undefined" && app && app.fontScale) ? app.fontScale : 1.0))
            color: Theme.rgba(Theme.secondaryColor, 0.6)
            wrapMode: Text.Wrap
        }
    }

    Component {
        id: headingComponent
        InlineText {
            property int level: blockData.level || 1
            spans: blockData.spans || []
            color: Theme.highlightColor
            font.family: (typeof app !== "undefined" && app && app.docFontFamily && app.docFontFamily.length > 0) ? app.docFontFamily : Theme.fontFamily
            font.pixelSize: {
                var scale = (typeof app !== "undefined" && app && app.fontScale) ? app.fontScale : 1.0
                switch (level) {
                    case 1: return Math.round(Theme.fontSizeExtraLarge * scale)
                    case 2: return Math.round(Theme.fontSizeLarge * scale)
                    case 3: return Math.round(Theme.fontSizeMedium * scale)
                    default: return Math.round(Theme.fontSizeSmall * scale)
                }
            }
            font.bold: level <= 2
            wrapMode: Text.Wrap
            x: Theme.horizontalPageMargin
            width: parent.width - Theme.horizontalPageMargin * 2
            onXrefActivated: function(target) {
                delegate.xrefActivated(target)
            }
        }
    }

    Component {
        id: paragraphComponent
        InlineText {
            spans: blockData.spans || []
            x: Theme.horizontalPageMargin
            width: parent.width - Theme.horizontalPageMargin * 2
            font.family: (typeof app !== "undefined" && app && app.docFontFamily && app.docFontFamily.length > 0) ? app.docFontFamily : Theme.fontFamily
            font.pixelSize: Math.round(Theme.fontSizeMedium * ((typeof app !== "undefined" && app && app.fontScale) ? app.fontScale : 1.0))
            onXrefActivated: function(target) {
                delegate.xrefActivated(target)
            }
        }
    }

    // Shared HTML generation for list rendering
    function escapeHtml(text) {
        return text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;")
    }

    function spansToPlainText(spans) {
        if (!spans) return ""
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

    function resolveImagePath(target) {
        if (!target) return ""
        if (target.indexOf("://") !== -1 || target.indexOf("/") === 0) {
            return target
        }
        var notesDir = (typeof bridge !== "undefined" && bridge && bridge.notes_dir) ? bridge.notes_dir : ""
        if (notesDir.length > 0) {
            return "file://" + notesDir + "/" + target
        }
        return "file:///usr/share/harbour-fishdoc/examples/" + target
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

    function spanToHtml(span) {
        if (!span) return ""
        switch (span.type) {
            case "text": return escapeHtml(span.value || "")
            case "bold": return "<b>" + spansToHtml(span.spans || []) + "</b>"
            case "italic": return "<i>" + spansToHtml(span.spans || []) + "</i>"
            case "code": return "<code style='background:#18181c;color:#f2f2f7;padding:1px 4px;border-radius:3px;font-family:monospace;'>" + escapeHtml(span.value || "") + "</code>"
            case "link": return "<a href='" + escapeHtml(span.url || "") + "' style='color:" + Theme.highlightColor + "'>" + escapeHtml(span.display || span.url || "") + "</a>"
            case "xref": return "<a href='xref:" + escapeHtml(span.target || "") + "' style='color:" + Theme.highlightColor + "'>" + escapeHtml(span.display || span.target || "") + "</a>"
            case "strikethrough": return "<s>" + spansToHtml(span.spans || []) + "</s>"
            case "superscript": return "<sup>" + spansToHtml(span.spans || []) + "</sup>"
            case "subscript": return "<sub>" + spansToHtml(span.spans || []) + "</sub>"
            case "image":
                var wAttr = span.width ? (" width='" + escapeHtml(span.width) + "'") : ""
                var src = resolveImagePath(span.target || "")
                return "<img src='" + escapeHtml(src) + "'" + wAttr + " alt='" + escapeHtml(span.alt || "") + "' />"
            case "icon":
                return renderIcon(span.name || "", span.options || "")
            case "footnote":
                return "<sup style='color:" + Theme.highlightColor + "'>[" + escapeHtml(span.text || span.id || "") + "]</sup>"
            case "callout":
                return "<b style='color:" + Theme.highlightColor + ";background:" + Theme.rgba(Theme.highlightBackgroundColor, 0.25) + ";padding:1px 4px;border-radius:8px;'>&lt;" + (span.number || 1) + "&gt;</b>"
            case "mark":
                return "<mark style='background:" + Theme.rgba(Theme.highlightColor, 0.25) + ";color:" + Theme.primaryColor + ";padding:1px 3px;border-radius:2px;'>" + spansToHtml(span.spans || []) + "</mark>"
            case "kbd":
                return "<kbd style='background:#2a2a2e;color:#f2f2f7;padding:1px 5px;border:1px solid #444;border-radius:4px;font-family:monospace;'>" + escapeHtml((span.keys || []).join("+")) + "</kbd>"
            case "btn":
                return "<span style='background:" + Theme.rgba(Theme.highlightColor, 0.2) + ";color:" + Theme.highlightColor + ";padding:1px 6px;border:1px solid " + Theme.highlightColor + ";border-radius:4px;font-weight:bold;'>[" + escapeHtml(span.text || "") + "]</span>"
            case "menu":
                return "<b style='color:" + Theme.highlightColor + ";'>" + escapeHtml(span.items ? span.items.join(" &#9656; ") : "") + "</b>"
            case "pass":
                return span.value || ""
            default: return escapeHtml(span.value || "")
        }
    }

    function spansToHtml(spans) {
        if (!spans) return ""
        var html = ""
        for (var i = 0; i < spans.length; i++) html += spanToHtml(spans[i])
        return html
    }

    function blocksToHtml(blocks, depth, pathPrefix) {
        if (!blocks) return ""
        if (depth === undefined) depth = 0
        if (pathPrefix === undefined) pathPrefix = ""

        var items = []
        for (var i = 0; i < blocks.length; i++) {
            items.push({ block: blocks[i], path: pathPrefix })
        }
        return blocksToHtmlFromItems(items, depth)
    }

    function blocksToHtmlFromItems(items, depth) {
        if (!items || items.length === 0) return ""
        if (depth === undefined) depth = 0
        var html = ""
        var listBuf = []
        var listOrdered = false

        function flushList() {
            if (listBuf.length === 0) return
            for (var i = 0; i < listBuf.length; i++) {
                var item = listBuf[i]
                var b = item.block
                var itemPath = item.path
                if (!b) continue
                var itemText = ""
                if (b.blocks && b.blocks.length > 0 && b.blocks[0].spans) {
                    itemText = spansToHtml(b.blocks[0].spans)
                }
                var isChecked = b.checked
                var checkMark = ""
                if (isChecked === true) checkMark = "\u2611"
                else if (isChecked === false) checkMark = "\u2610"

                var markerHtml = ""
                if (checkMark) {
                    markerHtml = "<a href='toggle:" + itemPath + "' style='color:" + Theme.primaryColor + ";text-decoration:none'>" + checkMark + "</a>"
                    if (itemText.indexOf("<a ") === -1) {
                        itemText = "<a href='toggle:" + itemPath + "' style='color:" + Theme.primaryColor + ";text-decoration:none'>" + itemText + "</a>"
                    }
                } else if (listOrdered) {
                    var m = (b.marker && b.marker.length > 0) ? b.marker : ((i + 1) + ".")
                    markerHtml = escapeHtml(m)
                } else {
                    markerHtml = "&bull;"
                }

                var childHtml = ""
                if (b.blocks && b.blocks.length > 1) {
                    var childList = []
                    for (var k = 1; k < b.blocks.length; k++) {
                        var childPath = itemPath ? (itemPath + "." + k) : ("" + k)
                        childList.push({ block: b.blocks[k], path: childPath })
                    }
                    childHtml = blocksToHtmlFromItems(childList, depth + 1)
                }

                html += "<table width='100%' border='0' cellpadding='0' cellspacing='0'>"
                html += "<tr><td valign='top' width='1%' style='padding-right:8px;white-space:nowrap;color:" + Theme.primaryColor + "'>" + markerHtml + "</td>"
                html += "<td width='99%'>" + itemText + childHtml + "</td></tr></table>"
            }
            listBuf = []
        }

        for (var i = 0; i < items.length; i++) {
            var it = items[i]
            var b = it.block
            if (!b) continue
            if (b.type === "unordered_list_item") {
                if (listOrdered || listBuf.length === 0) { flushList(); listOrdered = false }
                listBuf.push(it)
            } else if (b.type === "ordered_list_item") {
                if (!listOrdered || listBuf.length === 0) { flushList(); listOrdered = true }
                listBuf.push(it)
            } else {
                flushList()
                if (b.type === "paragraph" && b.spans) {
                    html += "<p>" + spansToHtml(b.spans) + "</p>"
                } else if (b.type === "code_block" && b.lines) {
                    html += "<div><tt style='background:#18181c;color:#f2f2f7;border-radius:4px;display:block;padding:6px;font-family:monospace;'>" + escapeHtml(b.lines.join("\n")) + "</tt></div>"
                }
            }
        }
        flushList()
        return html
    }

    Component {
        id: orderedListComponent
        InlineText {
            text: (delegate.renderCounter >= 0) ? blocksToHtml([localBlockData], 0, "") : ""
            font.family: (typeof app !== "undefined" && app && app.docFontFamily && app.docFontFamily.length > 0) ? app.docFontFamily : Theme.fontFamily
            font.pixelSize: Math.round(Theme.fontSizeSmall * ((typeof app !== "undefined" && app && app.fontScale) ? app.fontScale : 1.0))
            x: Theme.horizontalPageMargin
            width: parent.width - Theme.horizontalPageMargin * 2
            blockIndex: delegate.blockIndex
            onXrefActivated: function(target) { delegate.xrefActivated(target) }
            onCheckboxToggled: function(idx, itemPath) {
                delegate.toggleLocalCheckbox(itemPath)
                delegate.checkboxToggled(idx, itemPath)
            }
        }
    }

    Component {
        id: unorderedListComponent
        InlineText {
            text: (delegate.renderCounter >= 0) ? blocksToHtml([localBlockData], 0, "") : ""
            font.family: (typeof app !== "undefined" && app && app.docFontFamily && app.docFontFamily.length > 0) ? app.docFontFamily : Theme.fontFamily
            font.pixelSize: Math.round(Theme.fontSizeSmall * ((typeof app !== "undefined" && app && app.fontScale) ? app.fontScale : 1.0))
            x: Theme.horizontalPageMargin
            width: parent.width - Theme.horizontalPageMargin * 2
            blockIndex: delegate.blockIndex
            onXrefActivated: function(target) { delegate.xrefActivated(target) }
            onCheckboxToggled: function(idx, itemPath) {
                delegate.toggleLocalCheckbox(itemPath)
                delegate.checkboxToggled(idx, itemPath)
            }
        }
    }

    Component {
        id: codeBlockComponent
        Rectangle {
            color: "#18181c"
            border.color: Theme.rgba(Theme.primaryColor, 0.2)
            border.width: 1
            radius: Theme.paddingSmall
            height: codeCol.height + Theme.paddingMedium * 2
            x: Theme.horizontalPageMargin
            width: parent.width - Theme.horizontalPageMargin * 2

            Column {
                id: codeCol
                anchors {
                    left: parent.left
                    right: parent.right
                    top: parent.top
                    margins: Theme.paddingMedium
                }
                spacing: Theme.paddingSmall / 2

                Label {
                    visible: (blockData.language || "").length > 0
                    text: (blockData.language || "").toUpperCase()
                    font.family: "monospace"
                    font.pixelSize: Math.round(Theme.fontSizeExtraSmall * (typeof app !== "undefined" && app && app.codeFontScale ? app.codeFontScale : 1.0))
                    color: Theme.rgba("#f2f2f7", 0.6)
                }

                Label {
                    id: codeLabel
                    width: parent.width
                    text: (blockData.lines || []).join("\n")
                    font.family: "monospace"
                    font.pixelSize: Math.round(Theme.fontSizeSmall * (typeof app !== "undefined" && app && app.codeFontScale ? app.codeFontScale : 1.0))
                    color: "#f2f2f7"
                    wrapMode: Text.Wrap
                }
            }
        }
    }

    Component {
        id: literalBlockComponent
        Rectangle {
            color: "#18181c"
            border.color: Theme.rgba(Theme.primaryColor, 0.2)
            border.width: 1
            radius: Theme.paddingSmall
            height: litLabel.height + Theme.paddingMedium * 2
            x: Theme.horizontalPageMargin
            width: parent.width - Theme.horizontalPageMargin * 2

            Label {
                id: litLabel
                anchors {
                    left: parent.left
                    right: parent.right
                    margins: Theme.paddingMedium
                    verticalCenter: parent.verticalCenter
                }
                text: (blockData.lines || []).join("\n")
                font.family: "monospace"
                font.pixelSize: Math.round(Theme.fontSizeSmall * (typeof app !== "undefined" && app && app.codeFontScale ? app.codeFontScale : 1.0))
                color: "#f2f2f7"
                wrapMode: Text.Wrap
            }
        }
    }

    Component {
        id: blockquoteComponent
        Rectangle {
            color: "transparent"
            border.color: Theme.highlightColor
            border.width: 2
            radius: 4
            height: quoteLabel.height + Theme.paddingSmall * 2
            x: Theme.horizontalPageMargin
            width: parent.width - Theme.horizontalPageMargin * 2

            Label {
                id: quoteLabel
                anchors {
                    left: parent.left
                    right: parent.right
                    margins: Theme.paddingMedium
                    verticalCenter: parent.verticalCenter
                }
                text: blockData.raw || ""
                font.italic: true
                font.family: (typeof app !== "undefined" && app && app.docFontFamily && app.docFontFamily.length > 0) ? app.docFontFamily : Theme.fontFamily
                font.pixelSize: Math.round(Theme.fontSizeMedium * ((typeof app !== "undefined" && app && app.fontScale) ? app.fontScale : 1.0))
                color: Theme.secondaryColor
                wrapMode: Text.Wrap
            }
        }
    }

    Component {
        id: tableComponent
        Column {
            x: Theme.horizontalPageMargin
            width: parent.width - Theme.horizontalPageMargin * 2
            spacing: 2

            Column {
                id: tableGrid
                property int columnCount: (blockData.rows && blockData.rows[0]) ? blockData.rows[0].length : 1
                property int rowCount: blockData.rows ? blockData.rows.length : 0
                property string frameAttr: (blockData.frame || "all").toLowerCase()
                property string gridAttr: (blockData.grid || "all").toLowerCase()

                property bool hasTopBorder: frameAttr === "all" || frameAttr === "topbot" || frameAttr === "rows" || frameAttr === "ends"
                property bool hasBottomBorder: frameAttr === "all" || frameAttr === "topbot" || frameAttr === "rows" || frameAttr === "ends"
                property bool hasLeftBorder: frameAttr === "all" || frameAttr === "sides" || frameAttr === "cols"
                property bool hasRightBorder: frameAttr === "all" || frameAttr === "sides" || frameAttr === "cols"

                property bool hasRowDividers: gridAttr === "all" || gridAttr === "rows" || gridAttr === "horizontal"
                property bool hasColDividers: gridAttr === "all" || gridAttr === "cols" || gridAttr === "vertical"

                property color borderColor: Theme.rgba(Theme.primaryColor, 0.25)
                property color headerBgColor: Theme.rgba(Theme.highlightBackgroundColor, 0.12)

                property var colWidths: {
                    var cw = blockData.col_widths || []
                    var total = 0
                    for (var i = 0; i < columnCount; i++) total += (cw[i] || 1)
                    var result = []
                    for (var j = 0; j < columnCount; j++) result.push((cw[j] || 1) / total)
                    return result
                }
                width: parent.width
                spacing: 0

                Repeater {
                    model: blockData.rows || []

                    delegate: Row {
                        property int rowIndex: index
                        property var row: modelData
                        spacing: 0
                        width: parent.width

                        Repeater {
                            model: {
                                var cells = []
                                for (var c = 0; c < row.length; c++) {
                                    cells.push({ blocks: row[c], col: c, header: rowIndex === 0 })
                                }
                                return cells
                            }

                            delegate: Item {
                                property int colIndex: modelData.col || 0
                                property bool isHeader: modelData.header
                                width: {
                                    if (colIndex === tableGrid.columnCount - 1) {
                                        var used = 0
                                        for (var k = 0; k < colIndex; k++) {
                                            used += Math.floor(tableGrid.colWidths[k] * tableGrid.width)
                                        }
                                        return Math.max(0, tableGrid.width - used)
                                    }
                                    return Math.floor(tableGrid.colWidths[colIndex] * tableGrid.width)
                                }
                                height: Math.max(48, cellText.paintedHeight + 16)

                                Rectangle {
                                    anchors.fill: parent
                                    color: isHeader ? tableGrid.headerBgColor : "transparent"
                                }

                                // Top border line
                                Rectangle {
                                    anchors.top: parent.top
                                    anchors.left: parent.left
                                    anchors.right: parent.right
                                    height: 1
                                    color: tableGrid.borderColor
                                    visible: (rowIndex === 0 && tableGrid.hasTopBorder) || (rowIndex > 0 && tableGrid.hasRowDividers)
                                }

                                // Bottom border line (only for last row's bottom frame border)
                                Rectangle {
                                    anchors.bottom: parent.bottom
                                    anchors.left: parent.left
                                    anchors.right: parent.right
                                    height: 1
                                    color: tableGrid.borderColor
                                    visible: rowIndex === tableGrid.rowCount - 1 && tableGrid.hasBottomBorder
                                }

                                // Left border line
                                Rectangle {
                                    anchors.left: parent.left
                                    anchors.top: parent.top
                                    anchors.bottom: parent.bottom
                                    width: 1
                                    color: tableGrid.borderColor
                                    visible: (colIndex === 0 && tableGrid.hasLeftBorder) || (colIndex > 0 && tableGrid.hasColDividers)
                                }

                                // Right border line (only for last column's right frame border)
                                Rectangle {
                                    anchors.right: parent.right
                                    anchors.top: parent.top
                                    anchors.bottom: parent.bottom
                                    width: 1
                                    color: tableGrid.borderColor
                                    visible: colIndex === tableGrid.columnCount - 1 && tableGrid.hasRightBorder
                                }

                                Label {
                                    id: cellText
                                    anchors {
                                        left: parent.left
                                        right: parent.right
                                        top: parent.top
                                        leftMargin: 8
                                        rightMargin: 8
                                        topMargin: 8
                                    }
                                    wrapMode: Text.Wrap
                                    font.family: (typeof app !== "undefined" && app && app.docFontFamily && app.docFontFamily.length > 0) ? app.docFontFamily : Theme.fontFamily
                                    font.pixelSize: Math.round(Theme.fontSizeSmall * ((typeof app !== "undefined" && app && app.fontScale) ? app.fontScale : 1.0))
                                    font.bold: modelData.header
                                    color: Theme.primaryColor
                                    linkColor: Theme.highlightColor
                                    textFormat: Text.RichText
                                    text: {
                                        var blocks = modelData.blocks || []
                                        if (typeof blocks === "string") return blocks
                                        return blocksToHtml(blocks, 0)
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Component {
        id: admonitionComponent
        Item {
            property string kind: blockData.kind || "NOTE"
            property color borderColor: {
                switch (kind) {
                    case "WARNING": return "#d9534f"
                    case "TIP": return "#f0ad4e"
                    default: return Theme.highlightColor
                }
            }
            property color bgColor: {
                switch (kind) {
                    case "WARNING": return Qt.rgba(0.85, 0.33, 0.31, 0.12)
                    case "TIP": return Qt.rgba(0.94, 0.68, 0.31, 0.12)
                    default: return Qt.rgba(Theme.highlightBackgroundColor.r, Theme.highlightBackgroundColor.g, Theme.highlightBackgroundColor.b, 0.12)
                }
            }
            property string iconSource: {
                switch (kind) {
                    case "WARNING": return "image://theme/icon-s-warning?" + borderColor
                    case "TIP": return "image://theme/icon-s-high-importance?" + borderColor
                    default: return "image://theme/icon-m-about?" + borderColor
                }
            }

            anchors.left: parent.left
            anchors.right: parent.right
            anchors.leftMargin: Theme.horizontalPageMargin
            anchors.rightMargin: Theme.horizontalPageMargin
            height: admonitionRect.height

            Rectangle {
                id: admonitionRect
                anchors.left: parent.left
                anchors.right: parent.right
                color: bgColor
                border.color: borderColor
                border.width: 2
                radius: 4
                height: Math.max(iconImage.height + Theme.paddingMedium * 2, admonitionColumn.height + Theme.paddingMedium * 2)

                Image {
                    id: iconImage
                    anchors {
                        left: parent.left
                        top: parent.top
                        margins: Theme.paddingMedium
                    }
                    source: iconSource
                    width: Theme.iconSizeSmall
                    height: Theme.iconSizeSmall
                    sourceSize.width: Theme.iconSizeSmall
                    sourceSize.height: Theme.iconSizeSmall
                }

                Column {
                    id: admonitionColumn
                    anchors {
                        left: iconImage.right
                        right: parent.right
                        top: parent.top
                        margins: Theme.paddingMedium
                        leftMargin: Theme.paddingMedium
                    }
                    spacing: Theme.paddingSmall

                    Label {
                        text: kind
                        font.bold: true
                        font.family: (typeof app !== "undefined" && app && app.docFontFamily && app.docFontFamily.length > 0) ? app.docFontFamily : Theme.fontFamily
                        font.pixelSize: Math.round(Theme.fontSizeSmall * ((typeof app !== "undefined" && app && app.fontScale) ? app.fontScale : 1.0))
                        color: borderColor
                    }

                    InlineText {
                        text: blocksToHtml(blockData.blocks || [], 0)
                        width: parent.width
                        font.pixelSize: Math.round(Theme.fontSizeSmall * ((typeof app !== "undefined" && app && app.fontScale) ? app.fontScale : 1.0))
                    }
                }
            }
        }
    }

    Component {
        id: tocComponent
        Item {
            id: tocWrapper
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.leftMargin: Theme.horizontalPageMargin
            anchors.rightMargin: Theme.horizontalPageMargin
            height: tocRect.height

            Rectangle {
                id: tocRect
                anchors.left: parent.left
                anchors.right: parent.right
                height: tocColumn.height + Theme.paddingMedium * 2
                color: Qt.rgba(Theme.highlightBackgroundColor.r, Theme.highlightBackgroundColor.g, Theme.highlightBackgroundColor.b, 0.1)
                border.color: Theme.highlightColor
                border.width: 1
                radius: 4
                clip: true

                Column {
                    id: tocColumn
                    anchors {
                        left: parent.left
                        right: parent.right
                        margins: Theme.paddingMedium
                        top: parent.top
                        topMargin: Theme.paddingMedium
                    }
                    spacing: Theme.paddingSmall

                    BackgroundItem {
                        id: tocHeader
                        width: parent.width
                        height: Math.max(Theme.itemSizeExtraSmall * 0.7, titleLabel.height + Theme.paddingSmall * 2)

                        Label {
                            id: titleLabel
                            anchors.left: parent.left
                            anchors.right: chevronIcon.left
                            anchors.rightMargin: Theme.paddingSmall
                            anchors.verticalCenter: parent.verticalCenter
                            text: "Table of Contents"
                            font.bold: true
                            font.family: (typeof app !== "undefined" && app && app.docFontFamily && app.docFontFamily.length > 0) ? app.docFontFamily : Theme.fontFamily
                            font.pixelSize: Math.round(Theme.fontSizeSmall * ((typeof app !== "undefined" && app && app.fontScale) ? app.fontScale : 1.0))
                            color: tocHeader.highlighted ? Theme.primaryColor : Theme.highlightColor
                            truncationMode: TruncationMode.Fade
                        }

                        Image {
                            id: chevronIcon
                            anchors.right: parent.right
                            anchors.verticalCenter: parent.verticalCenter
                            source: "image://theme/icon-s-dropdown?" + (tocHeader.highlighted ? Theme.primaryColor : Theme.highlightColor)
                            transformOrigin: Item.Center
                            rotation: delegate.isTocCollapsed ? -90 : 0
                            Behavior on rotation { NumberAnimation { duration: 150 } }
                        }

                        onClicked: {
                            delegate.toggleToc(delegate.blockIndex)
                        }
                    }

                    Column {
                        id: entriesColumn
                        width: parent.width
                        spacing: 2
                        visible: !delegate.isTocCollapsed

                        property var tocList: {
                            if (delegate.blockData && delegate.blockData.headings && delegate.blockData.headings.length > 0) {
                                return delegate.blockData.headings
                            }
                            var headings = []
                            var blocks = delegate.allBlocks || []
                            for (var i = 0; i < blocks.length; i++) {
                                var b = blocks[i]
                                if (b && b.type === "heading" && b.level >= 1 && b.level <= 5) {
                                    headings.push({
                                        level: b.level,
                                        text: spansToPlainText(b.spans || []),
                                        index: i
                                    })
                                }
                            }
                            return headings
                        }

                        property int minLevel: {
                            var minL = 99
                            for (var i = 0; i < tocList.length; i++) {
                                if (tocList[i] && tocList[i].level && tocList[i].level < minL) {
                                    minL = tocList[i].level
                                }
                            }
                            return minL === 99 ? 1 : minL
                        }

                        Repeater {
                            id: tocRepeater
                            model: entriesColumn.tocList

                            delegate: BackgroundItem {
                                id: tocEntry
                                property var headingItem: modelData || ({})
                                property int headingLevel: headingItem && headingItem.level ? headingItem.level : 1
                                property string headingText: headingItem && headingItem.text ? headingItem.text : ""
                                property int headingIndex: headingItem && headingItem.index !== undefined ? headingItem.index : -1
                                property int indent: Math.max(0, (headingLevel - entriesColumn.minLevel)) * Theme.paddingLarge
                                width: parent.width
                                height: headingLabel.height + Theme.paddingSmall * 2

                                Label {
                                    id: headingLabel
                                    anchors.left: parent.left
                                    anchors.right: parent.right
                                    anchors.leftMargin: indent
                                    anchors.rightMargin: Theme.paddingSmall
                                    anchors.verticalCenter: parent.verticalCenter
                                    text: "• " + headingText
                                    font.family: (typeof app !== "undefined" && app && app.docFontFamily && app.docFontFamily.length > 0) ? app.docFontFamily : Theme.fontFamily
                                    font.pixelSize: Math.round(Theme.fontSizeSmall * ((typeof app !== "undefined" && app && app.fontScale) ? app.fontScale : 1.0))
                                    color: tocEntry.highlighted ? Theme.primaryColor : Theme.highlightColor
                                    wrapMode: Text.Wrap
                                }

                                onClicked: {
                                    if (headingIndex >= 0) {
                                        delegate.jumpToBlock(headingIndex)
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Component {
        id: imageComponent
        Item {
            anchors.left: parent ? parent.left : undefined
            anchors.right: parent ? parent.right : undefined
            anchors.leftMargin: Theme.horizontalPageMargin
            anchors.rightMargin: Theme.horizontalPageMargin
            height: img.status === Image.Ready ? (img.height + (captionLabel.visible ? captionLabel.height + Theme.paddingSmall : 0) + Theme.paddingMedium) : (errorLabel.visible ? (errorLabel.height + Theme.paddingMedium) : Theme.itemSizeMedium)

            Image {
                id: img
                anchors.top: parent.top
                anchors.horizontalCenter: parent.horizontalCenter
                width: {
                    if (blockData.width) {
                        var w = parseInt(blockData.width, 10)
                        if (!isNaN(w) && w > 0) return Math.min(w, parent.width)
                    }
                    return Math.min(implicitWidth > 0 ? implicitWidth : parent.width, parent.width)
                }
                fillMode: Image.PreserveAspectFit
                source: resolveImagePath(blockData.target || "")
                asynchronous: true
            }

            Label {
                id: captionLabel
                anchors.top: img.bottom
                anchors.topMargin: Theme.paddingSmall
                anchors.horizontalCenter: parent.horizontalCenter
                width: parent.width
                horizontalAlignment: Text.AlignHCenter
                font.pixelSize: Theme.fontSizeExtraSmall
                color: Theme.secondaryColor
                text: blockData.alt || ""
                visible: text.length > 0 && img.status === Image.Ready
                wrapMode: Text.Wrap
            }

            Label {
                id: errorLabel
                anchors.centerIn: parent
                width: parent.width
                horizontalAlignment: Text.AlignHCenter
                font.pixelSize: Theme.fontSizeSmall
                color: Theme.secondaryColor
                text: blockData.alt ? (blockData.alt + " (" + (blockData.target || "") + ")") : (blockData.target || "Image")
                visible: img.status === Image.Error || img.status === Image.Null
                wrapMode: Text.Wrap
            }
        }
    }

    Component {
        id: sidebarComponent
        Item {
            anchors.left: parent ? parent.left : undefined
            anchors.right: parent ? parent.right : undefined
            anchors.leftMargin: Theme.horizontalPageMargin
            anchors.rightMargin: Theme.horizontalPageMargin
            height: sidebarRect.height

            Rectangle {
                id: sidebarRect
                anchors.left: parent.left
                anchors.right: parent.right
                height: sidebarCol.height + Theme.paddingMedium * 2
                color: Theme.rgba(Theme.highlightBackgroundColor, 0.08)
                border.color: Theme.rgba(Theme.highlightColor, 0.35)
                border.width: 1
                radius: Theme.paddingSmall

                // Dedicated left accent stripe for sidebar callout
                Rectangle {
                    anchors.left: parent.left
                    anchors.top: parent.top
                    anchors.bottom: parent.bottom
                    width: 4
                    color: Theme.highlightColor
                    radius: Theme.paddingSmall
                }

                Column {
                    id: sidebarCol
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: parent.top
                    anchors.leftMargin: Theme.paddingMedium + 6
                    anchors.rightMargin: Theme.paddingMedium
                    anchors.topMargin: Theme.paddingMedium
                    spacing: Theme.paddingSmall

                    Row {
                        width: parent.width
                        spacing: Theme.paddingSmall
                        visible: (blockData.title || "").length > 0

                        Rectangle {
                            width: Theme.paddingSmall
                            height: Theme.paddingSmall
                            radius: Theme.paddingSmall / 2
                            color: Theme.highlightColor
                            anchors.verticalCenter: parent.verticalCenter
                        }

                        Label {
                            width: parent.width - Theme.paddingSmall * 2
                            text: blockData.title || ""
                            color: Theme.highlightColor
                            font.bold: true
                            font.family: (typeof app !== "undefined" && app && app.docFontFamily && app.docFontFamily.length > 0) ? app.docFontFamily : Theme.fontFamily
                            font.pixelSize: Math.round(Theme.fontSizeSmall * ((typeof app !== "undefined" && app && app.fontScale) ? app.fontScale : 1.0))
                            font.capitalization: Font.AllUppercase
                            wrapMode: Text.Wrap
                        }
                    }

                    InlineText {
                        width: parent.width
                        text: blocksToHtml(blockData.blocks || [])
                        font.pixelSize: Math.round(Theme.fontSizeSmall * ((typeof app !== "undefined" && app && app.fontScale) ? app.fontScale : 1.0))
                        onXrefActivated: function(target) { delegate.xrefActivated(target) }
                        onCheckboxToggled: function(idx, path) { delegate.checkboxToggled(delegate.blockIndex, path) }
                    }
                }
            }
        }
    }

    Component {
        id: exampleComponent
        Item {
            anchors.left: parent ? parent.left : undefined
            anchors.right: parent ? parent.right : undefined
            anchors.leftMargin: Theme.horizontalPageMargin
            anchors.rightMargin: Theme.horizontalPageMargin
            height: exampleRect.height

            Rectangle {
                id: exampleRect
                anchors.left: parent.left
                anchors.right: parent.right
                height: exampleCol.height + Theme.paddingMedium * 2
                color: Theme.rgba(Theme.primaryColor, 0.03)
                border.color: Theme.rgba(Theme.primaryColor, 0.25)
                border.width: 1
                radius: Theme.paddingSmall

                Column {
                    id: exampleCol
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: parent.top
                    anchors.margins: Theme.paddingMedium
                    spacing: Theme.paddingSmall

                    Label {
                        width: parent.width
                        text: blockData.title || "EXAMPLE"
                        color: Theme.secondaryColor
                        font.bold: true
                        font.family: (typeof app !== "undefined" && app && app.docFontFamily && app.docFontFamily.length > 0) ? app.docFontFamily : Theme.fontFamily
                        font.pixelSize: Math.round(Theme.fontSizeExtraSmall * ((typeof app !== "undefined" && app && app.fontScale) ? app.fontScale : 1.0))
                        font.capitalization: Font.AllUppercase
                        wrapMode: Text.Wrap
                    }

                    InlineText {
                        width: parent.width
                        text: blocksToHtml(blockData.blocks || [])
                        font.pixelSize: Math.round(Theme.fontSizeSmall * ((typeof app !== "undefined" && app && app.fontScale) ? app.fontScale : 1.0))
                        onXrefActivated: function(target) { delegate.xrefActivated(target) }
                        onCheckboxToggled: function(idx, path) { delegate.checkboxToggled(delegate.blockIndex, path) }
                    }
                }
            }
        }
    }

    Component {
        id: descriptionListComponent
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
                    text: blockData.term_spans ? ("<b>" + spansToHtml(blockData.term_spans) + "</b>") : ("<b>" + escapeHtml(blockData.term || "") + "</b>")
                    color: Theme.primaryColor
                    font.family: (typeof app !== "undefined" && app && app.docFontFamily && app.docFontFamily.length > 0) ? app.docFontFamily : Theme.fontFamily
                    font.pixelSize: Math.round(Theme.fontSizeMedium * ((typeof app !== "undefined" && app && app.fontScale) ? app.fontScale : 1.0))
                    wrapMode: Text.Wrap
                }

                InlineText {
                    width: parent.width - Theme.paddingMedium
                    x: Theme.paddingMedium
                    text: blocksToHtml(blockData.blocks || [])
                    font.pixelSize: Math.round(Theme.fontSizeSmall * ((typeof app !== "undefined" && app && app.fontScale) ? app.fontScale : 1.0))
                    onXrefActivated: function(target) { delegate.xrefActivated(target) }
                    onCheckboxToggled: function(idx, path) { delegate.checkboxToggled(delegate.blockIndex, path) }
                }
            }
        }
    }

    Component {
        id: calloutListComponent
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

            InlineText {
                id: calloutText
                anchors.left: badgeLabel.right
                anchors.leftMargin: Theme.paddingMedium
                anchors.right: parent.right
                font.pixelSize: Math.round(Theme.fontSizeSmall * ((typeof app !== "undefined" && app && app.fontScale) ? app.fontScale : 1.0))
                text: blocksToHtml(blockData.blocks || [])
                onXrefActivated: function(target) { delegate.xrefActivated(target) }
                onCheckboxToggled: function(idx, path) { delegate.checkboxToggled(delegate.blockIndex, path) }
            }
        }
    }

    Component {
        id: openComponent
        Column {
            x: Theme.horizontalPageMargin
            width: parent.width - Theme.horizontalPageMargin * 2
            spacing: Theme.paddingSmall

            Label {
                visible: (blockData.title || "").length > 0
                text: blockData.title || ""
                font.bold: true
                color: Theme.highlightColor
                font.pixelSize: Math.round(Theme.fontSizeSmall * ((typeof app !== "undefined" && app && app.fontScale) ? app.fontScale : 1.0))
            }

            InlineText {
                width: parent.width
                text: (delegate.renderCounter >= 0) ? blocksToHtml(blockData.blocks || [], 0, "") : ""
                font.family: (typeof app !== "undefined" && app && app.docFontFamily && app.docFontFamily.length > 0) ? app.docFontFamily : Theme.fontFamily
                font.pixelSize: Math.round(Theme.fontSizeMedium * ((typeof app !== "undefined" && app && app.fontScale) ? app.fontScale : 1.0))
                onXrefActivated: function(target) { delegate.xrefActivated(target) }
                onCheckboxToggled: function(idx, path) { delegate.checkboxToggled(delegate.blockIndex, path) }
            }
        }
    }

    Component {
        id: pageBreakComponent
        Item {
            x: Theme.horizontalPageMargin
            width: parent.width - Theme.horizontalPageMargin * 2
            height: Theme.paddingLarge

            Rectangle {
                anchors.centerIn: parent
                width: parent.width
                height: 1
                color: Theme.rgba(Theme.primaryColor, 0.2)
            }
        }
    }

    Component {
        id: hrComponent
        Separator {
            x: Theme.horizontalPageMargin
            width: parent.width - Theme.horizontalPageMargin * 2
            color: Theme.highlightColor
        }
    }

    Component {
        id: emptyComponent
        Item {
            height: Theme.paddingSmall
            width: parent.width
        }
    }
}
