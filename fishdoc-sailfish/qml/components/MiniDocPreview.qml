import QtQuick 2.6
import Sailfish.Silica 1.0

Item {
    id: miniDocPreview
    property string previewBlocksJson: ""
    property var rawBlocks: []
    property string snippet: ""
    property bool highlighted: false
    property bool showBorder: true
    property real contentScale: (typeof app !== "undefined" && app && app.previewScale !== undefined) ? app.previewScale : 0.52
    property real previewHeight: width  // square by default

    height: previewHeight

    property var parsedBlocks: {
        var list = []
        if (previewBlocksJson && previewBlocksJson.length > 0) {
            try {
                var arr = JSON.parse(previewBlocksJson)
                for (var i = 0; i < arr.length; i++) {
                    var itm = arr[i]
                    if (typeof itm === "string") {
                        try { list.push(JSON.parse(itm)) } catch(e) {}
                    } else if (typeof itm === "object" && itm !== null) {
                        list.push(itm)
                    }
                }
            } catch(e) {}
        }
        if (list.length === 0 && rawBlocks && rawBlocks.length > 0) {
            for (var j = 0; j < rawBlocks.length; j++) {
                var r = rawBlocks[j]
                if (typeof r === "string") {
                    try { list.push(JSON.parse(r)) } catch(e) {}
                } else if (typeof r === "object" && r !== null) {
                    list.push(r)
                }
            }
        }
        return list
    }

    function escapeHtml(text) {
        if (!text) return ""
        return String(text)
            .replace(/&/g, "&amp;")
            .replace(/</g, "&lt;")
            .replace(/>/g, "&gt;")
            .replace(/"/g, "&quot;")
            .replace(/'/g, "&#039;")
    }

    function spansToText(spans) {
        if (!spans || !spans.length) return ""
        var out = ""
        for (var i = 0; i < spans.length; i++) {
            var s = spans[i]
            if (s.value) out += s.value
            else if (s.spans) out += spansToText(s.spans)
            else if (s.display) out += s.display
            else if (s.target) out += s.target
            else if (s.text) out += s.text
            else if (s.alt) out += s.alt
        }
        return out
    }

    function spansToMiniHtml(spans) {
        if (!spans || !spans.length) return ""
        var out = ""
        for (var i = 0; i < spans.length; i++) {
            var s = spans[i]
            if (!s) continue
            switch (s.type) {
                case "bold":
                    out += "<b>" + spansToMiniHtml(s.spans) + "</b>"
                    break
                case "italic":
                    out += "<i>" + spansToMiniHtml(s.spans) + "</i>"
                    break
                case "code":
                    out += "<font color='#f2f2f7' style='background:#18181c;'>&nbsp;" + escapeHtml(s.value) + "&nbsp;</font>"
                    break
                case "link":
                    out += "<font color='" + Theme.highlightColor + "'>" + escapeHtml(s.display || s.url || "") + "</font>"
                    break
                case "xref":
                    out += "<font color='" + Theme.highlightColor + "'>" + escapeHtml(s.display || s.target || "") + "</font>"
                    break
                case "strikethrough":
                    out += "<s>" + spansToMiniHtml(s.spans) + "</s>"
                    break
                case "icon":
                    out += "<font color='" + Theme.highlightColor + "'>&#9670;</font>"
                    break
                case "callout":
                    out += "<font color='" + Theme.highlightColor + "'><b>&lt;" + (s.number || 1) + "&gt;</b></font>"
                    break
                case "mark":
                    out += "<mark style='background:" + Theme.rgba(Theme.highlightColor, 0.25) + ";color:" + Theme.primaryColor + ";padding:0 2px;'>" + spansToMiniHtml(s.spans) + "</mark>"
                    break
                case "kbd":
                    out += "<font color='#f2f2f7' style='background:#2a2a2e;'>&nbsp;" + escapeHtml((s.keys || []).join("+")) + "&nbsp;</font>"
                    break
                case "btn":
                    out += "<font color='" + Theme.highlightColor + "'>[" + escapeHtml(s.text || "") + "]</font>"
                    break
                case "menu":
                    out += "<font color='" + Theme.highlightColor + "'><b>" + escapeHtml((s.items || []).join(" &gt; ")) + "</b></font>"
                    break
                case "pass":
                    out += (s.value || "")
                    break
                default:
                    out += escapeHtml(s.value || "")
                    break
            }
        }
        return out
    }

    function getChildSpans(b) {
        if (!b) return []
        if (b.spans) return b.spans
        if (b.blocks && b.blocks.length > 0) {
            return getChildSpans(b.blocks[0])
        }
        return []
    }

    function getChildText(b) {
        if (!b) return ""
        if (b.text) return b.text
        if (b.raw_text) return b.raw_text
        if (b.spans) return spansToText(b.spans)
        if (b.blocks && b.blocks.length > 0) {
            return getChildText(b.blocks[0])
        }
        return ""
    }

    Rectangle {
        id: cardRect
        anchors.fill: parent
        radius: showBorder ? Theme.paddingSmall : 0
        color: showBorder ? (highlighted ? Theme.rgba(Theme.highlightColor, 0.12) : Theme.rgba(Theme.primaryColor, 0.04)) : "transparent"
        border.color: showBorder ? (highlighted ? Theme.rgba(Theme.highlightColor, 0.5) : Theme.rgba(Theme.primaryColor, 0.15)) : "transparent"
        border.width: showBorder ? 1 : 0
        clip: true

        Item {
            id: scaledWrapper
            x: showBorder ? Theme.paddingSmall : 0
            y: showBorder ? Theme.paddingSmall : 0
            width: Math.max(10, (cardRect.width - (showBorder ? Theme.paddingSmall * 2 : 0)) / miniDocPreview.contentScale)
            height: Math.max(10, (cardRect.height - (showBorder ? Theme.paddingSmall * 2 : 0)) / miniDocPreview.contentScale)
            scale: miniDocPreview.contentScale
            transformOrigin: Item.TopLeft

            Column {
                id: contentColumn
                width: parent.width
                spacing: 6

                // Fallback snippet when parsed blocks are empty
                Label {
                    width: parent.width
                    visible: parsedBlocks.length === 0 && snippet.length > 0
                    text: snippet
                    textFormat: Text.RichText
                    font.pixelSize: Theme.fontSizeSmall
                    color: Theme.secondaryColor
                    wrapMode: Text.Wrap
                    maximumLineCount: 6
                    truncationMode: TruncationMode.Elide
                }

                Label {
                    width: parent.width
                    visible: parsedBlocks.length === 0 && snippet.length === 0
                    text: "Empty document"
                    font.italic: true
                    font.pixelSize: Theme.fontSizeSmall
                    color: Theme.secondaryColor
                }

                // Live miniaturized rendered blocks
                Repeater {
                    model: parsedBlocks.slice(0, 10)

                    delegate: Item {
                        id: blockRowItem
                        width: contentColumn.width
                        height: {
                            var b = modelData
                            if (!b || !b.type) return 0
                            if (b.type === "heading") return headingRow.height
                            if (b.type === "paragraph") return paragraphLabel.height
                            if (b.type === "unordered_list_item" || b.type === "ordered_list_item") return listRow.height
                            if (b.type === "code_block" || b.type === "literal_block") return codeBox.height
                            if (b.type === "admonition") return admonBox.height
                            if (b.type === "blockquote") return quoteRow.height
                            if (b.type === "table") return tableBox.height
                            if (b.type === "image") return imageRow.height
                            if (b.type === "sidebar" || b.type === "example") return exBox.height
                            if (b.type === "open") return openBox.height
                            if (b.type === "page_break") return pageBreakLine.height
                            if (b.type === "description_list_item") return descRow.height
                            if (b.type === "comment") return commentRow.height
                            if (b.type === "toc") return miniTocBox.height
                            return paragraphLabel.height
                        }

                        property var blockData: modelData

                        // Heading
                        Row {
                            id: headingRow
                            width: parent.width
                            spacing: 6
                            visible: blockData && blockData.type === "heading"

                            Rectangle {
                                width: 3
                                height: Math.max(16, headingLabel.height)
                                color: Theme.highlightColor
                                radius: 1
                            }

                            Label {
                                id: headingLabel
                                width: parent.width - 9
                                text: blockData && blockData.spans ? spansToText(blockData.spans) : ""
                                color: Theme.highlightColor
                                font.bold: true
                                font.pixelSize: (blockData && blockData.level === 1) ? Theme.fontSizeMedium : Theme.fontSizeSmall
                                wrapMode: Text.Wrap
                                maximumLineCount: 2
                                truncationMode: TruncationMode.Elide
                            }
                        }

                        // Paragraph
                        Label {
                            id: paragraphLabel
                            width: parent.width
                            visible: blockData && blockData.type === "paragraph"
                            text: blockData && blockData.spans ? spansToMiniHtml(blockData.spans) : ""
                            textFormat: Text.RichText
                            color: Theme.primaryColor
                            font.pixelSize: Theme.fontSizeExtraSmall
                            wrapMode: Text.Wrap
                            maximumLineCount: 3
                            truncationMode: TruncationMode.Elide
                        }

                        // Lists
                        Row {
                            id: listRow
                            width: parent.width
                            spacing: 6
                            visible: blockData && (blockData.type === "unordered_list_item" || blockData.type === "ordered_list_item")

                            Label {
                                text: {
                                    if (!blockData) return "\u2022"
                                    if (blockData.checked === true) return "\u2611"
                                    if (blockData.checked === false) return "\u2610"
                                    if (blockData.type === "ordered_list_item") {
                                        return blockData.marker || "1."
                                    }
                                    return "\u2022"
                                }
                                color: Theme.highlightColor
                                font.pixelSize: Theme.fontSizeExtraSmall
                            }

                            Label {
                                width: parent.width - 24
                                text: spansToMiniHtml(getChildSpans(blockData))
                                textFormat: Text.RichText
                                color: Theme.primaryColor
                                font.pixelSize: Theme.fontSizeExtraSmall
                                wrapMode: Text.Wrap
                                maximumLineCount: 2
                                truncationMode: TruncationMode.Elide
                            }
                        }

                        // Code / Literal
                        Rectangle {
                            id: codeBox
                            width: parent.width
                            height: miniCodeText.height + 6
                            visible: blockData && (blockData.type === "code_block" || blockData.type === "literal_block")
                            color: "#18181c"
                            border.color: "#2a2a2e"
                            border.width: 1
                            radius: 3

                            Label {
                                id: miniCodeText
                                anchors.left: parent.left
                                anchors.right: parent.right
                                anchors.top: parent.top
                                anchors.margins: 4
                                text: (blockData && blockData.lines && blockData.lines.length > 0) ? blockData.lines.slice(0, 3).join("\n") : ""
                                font.family: "monospace"
                                font.pixelSize: Theme.fontSizeExtraSmall
                                color: "#f2f2f7"
                                maximumLineCount: 3
                                truncationMode: TruncationMode.Elide
                            }
                        }

                        // Admonition
                        Rectangle {
                            id: admonBox
                            width: parent.width
                            height: admonRow.height + 6
                            visible: blockData && blockData.type === "admonition"
                            color: Theme.rgba(Theme.highlightBackgroundColor, 0.15)
                            border.color: (blockData && (blockData.kind === "WARNING" || blockData.kind === "CAUTION")) ? "#e6a23c" : Theme.highlightColor
                            border.width: 1
                            radius: 3

                            Row {
                                id: admonRow
                                anchors.left: parent.left
                                anchors.right: parent.right
                                anchors.top: parent.top
                                anchors.margins: 4
                                spacing: 6

                                Label {
                                    text: "[" + (blockData ? (blockData.kind || "NOTE") : "NOTE") + "]"
                                    font.bold: true
                                    font.pixelSize: Theme.fontSizeExtraSmall
                                    color: (blockData && (blockData.kind === "WARNING" || blockData.kind === "CAUTION")) ? "#e6a23c" : Theme.highlightColor
                                }

                                Label {
                                    width: parent.width - 80
                                    text: getChildText(blockData)
                                    font.pixelSize: Theme.fontSizeExtraSmall
                                    color: Theme.primaryColor
                                    wrapMode: Text.Wrap
                                    maximumLineCount: 2
                                    truncationMode: TruncationMode.Elide
                                }
                            }
                        }

                        // Blockquote
                        Row {
                            id: quoteRow
                            width: parent.width
                            spacing: 6
                            visible: blockData && blockData.type === "blockquote"

                            Rectangle {
                                width: 3
                                height: Math.max(14, quoteLabel.height)
                                color: Theme.secondaryColor
                                radius: 1
                            }

                            Label {
                                id: quoteLabel
                                width: parent.width - 9
                                text: getChildText(blockData)
                                font.italic: true
                                font.pixelSize: Theme.fontSizeExtraSmall
                                color: Theme.secondaryColor
                                wrapMode: Text.Wrap
                                maximumLineCount: 2
                                truncationMode: TruncationMode.Elide
                            }
                        }

                        // Table
                        Rectangle {
                            id: tableBox
                            width: parent.width
                            height: 24
                            visible: blockData && blockData.type === "table"
                            color: Theme.rgba(Theme.primaryColor, 0.04)
                            border.color: Theme.rgba(Theme.primaryColor, 0.15)
                            border.width: 1
                            radius: 3

                            Label {
                                anchors.centerIn: parent
                                text: "\u229E Table" + (blockData && blockData.rows ? (" (" + blockData.rows.length + " rows)") : "")
                                font.pixelSize: Theme.fontSizeExtraSmall
                                color: Theme.secondaryColor
                            }
                        }

                        // Image
                        Row {
                            id: imageRow
                            width: parent.width
                            spacing: 6
                            visible: blockData && blockData.type === "image"

                            Label {
                                text: "&#128444;"
                                font.pixelSize: Theme.fontSizeSmall
                                color: Theme.highlightColor
                            }

                            Label {
                                width: parent.width - 24
                                text: blockData ? (blockData.alt || blockData.target || "Image") : "Image"
                                font.pixelSize: Theme.fontSizeExtraSmall
                                color: Theme.secondaryColor
                                wrapMode: Text.Wrap
                                maximumLineCount: 1
                                truncationMode: TruncationMode.Elide
                            }
                        }

                        // Sidebar / Example
                        Rectangle {
                            id: exBox
                            width: parent.width
                            height: exLabel.height + 6
                            visible: blockData && (blockData.type === "sidebar" || blockData.type === "example")
                            color: Theme.rgba(Theme.primaryColor, 0.04)
                            border.color: Theme.rgba(Theme.highlightColor, 0.3)
                            border.width: 1
                            radius: 3

                            Label {
                                id: exLabel
                                anchors.left: parent.left
                                anchors.right: parent.right
                                anchors.top: parent.top
                                anchors.margins: 4
                                text: blockData ? (blockData.title || getChildText(blockData) || "Example") : "Example"
                                font.pixelSize: Theme.fontSizeExtraSmall
                                color: Theme.primaryColor
                                wrapMode: Text.Wrap
                                maximumLineCount: 2
                                truncationMode: TruncationMode.Elide
                            }
                        }

                        // Description list
                        Row {
                            id: descRow
                            width: parent.width
                            spacing: 6
                            visible: blockData && blockData.type === "description_list_item"

                            Label {
                                text: blockData && blockData.term ? (spansToText(blockData.term) + ":") : ""
                                font.bold: true
                                font.pixelSize: Theme.fontSizeExtraSmall
                                color: Theme.highlightColor
                            }

                            Label {
                                width: parent.width - 80
                                text: getChildText(blockData)
                                font.pixelSize: Theme.fontSizeExtraSmall
                                color: Theme.primaryColor
                                wrapMode: Text.Wrap
                                maximumLineCount: 1
                                truncationMode: TruncationMode.Elide
                            }
                        }

                        // Open block
                        Rectangle {
                            id: openBox
                            width: parent.width
                            height: openText.height + 6
                            visible: blockData && blockData.type === "open"
                            color: Theme.rgba(Theme.primaryColor, 0.03)
                            border.color: Theme.rgba(Theme.primaryColor, 0.12)
                            border.width: 1
                            radius: 3

                            Label {
                                id: openText
                                anchors.left: parent.left
                                anchors.right: parent.right
                                anchors.top: parent.top
                                anchors.margins: 4
                                text: getChildText(blockData)
                                font.pixelSize: Theme.fontSizeExtraSmall
                                color: Theme.primaryColor
                                wrapMode: Text.Wrap
                                maximumLineCount: 2
                                truncationMode: TruncationMode.Elide
                            }
                        }

                        // Page break
                        Item {
                            id: pageBreakLine
                            width: parent.width
                            height: 12
                            visible: blockData && blockData.type === "page_break"

                            Rectangle {
                                anchors.centerIn: parent
                                width: parent.width
                                height: 1
                                color: Theme.rgba(Theme.secondaryColor, 0.3)
                            }
                        }

                        // Comment
                        Row {
                            id: commentRow
                            width: parent.width
                            spacing: 4
                            visible: blockData && blockData.type === "comment"

                            Label {
                                width: parent.width
                                text: "// " + (blockData ? (blockData.text || "") : "")
                                font.italic: true
                                font.family: "monospace"
                                font.pixelSize: Theme.fontSizeExtraSmall
                                color: Theme.rgba(Theme.secondaryColor, 0.6)
                                wrapMode: Text.Wrap
                                maximumLineCount: 1
                                truncationMode: TruncationMode.Elide
                            }
                        }

                        // Table of Contents
                        Rectangle {
                            id: miniTocBox
                            width: parent.width
                            height: miniTocCol.height + 8
                            visible: blockData && blockData.type === "toc"
                            color: Theme.rgba(Theme.highlightBackgroundColor, 0.15)
                            border.color: Theme.rgba(Theme.highlightColor, 0.4)
                            border.width: 1
                            radius: 3

                            property var headingList: (blockData && blockData.headings) ? blockData.headings : []
                            property bool isCollapsed: {
                                var threshold = (typeof app !== "undefined" && app && app.tocCollapseThreshold !== undefined) ? app.tocCollapseThreshold : 5;
                                if (threshold >= 20) return false;
                                if (threshold === 0) return true;
                                return headingList.length > threshold;
                            }

                            Column {
                                id: miniTocCol
                                anchors.left: parent.left
                                anchors.right: parent.right
                                anchors.top: parent.top
                                anchors.margins: 4
                                spacing: 3

                                Row {
                                    width: parent.width
                                    spacing: 4

                                    Label {
                                        text: "\u2261"
                                        color: Theme.highlightColor
                                        font.pixelSize: Theme.fontSizeExtraSmall
                                        font.bold: true
                                    }

                                    Label {
                                        text: "Table of Contents" + (miniTocBox.headingList.length > 0 ? (" (" + miniTocBox.headingList.length + ")") : "")
                                        color: Theme.highlightColor
                                        font.bold: true
                                        font.pixelSize: Theme.fontSizeExtraSmall
                                        width: parent.width - 36
                                        truncationMode: TruncationMode.Elide
                                    }

                                    Label {
                                        text: miniTocBox.isCollapsed ? "\u25BC" : "\u25B2"
                                        color: Theme.highlightColor
                                        font.pixelSize: Theme.fontSizeExtraSmall
                                    }
                                }

                                // Expanded list of headings
                                Repeater {
                                    model: miniTocBox.isCollapsed ? 0 : miniTocBox.headingList.slice(0, 4)
                                    delegate: Item {
                                        width: miniTocCol.width
                                        height: Math.max(12, miniTocEntryLabel.height)

                                        Label {
                                            id: miniTocEntryLabel
                                            x: Math.min(16, ((modelData.level || 1) - 1) * 6)
                                            width: parent.width - x
                                            text: modelData.text || ""
                                            color: Theme.highlightColor
                                            font.pixelSize: Theme.fontSizeExtraSmall
                                            truncationMode: TruncationMode.Elide
                                            maximumLineCount: 1
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Bottom subtle gradient fade
        Rectangle {
            anchors {
                left: parent.left
                right: parent.right
                bottom: parent.bottom
            }
            height: Theme.paddingLarge
            gradient: Gradient {
                GradientStop { position: 0.0; color: "transparent" }
                GradientStop {
                    position: 1.0
                    color: highlighted ? Theme.rgba(Theme.highlightColor, 0.25) : Theme.rgba(Theme.primaryColor, 0.1)
                }
            }
        }
    }
}
