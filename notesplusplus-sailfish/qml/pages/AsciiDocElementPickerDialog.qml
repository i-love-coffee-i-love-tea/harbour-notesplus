import QtQuick 2.6
import Sailfish.Silica 1.0

Dialog {
    id: elementPicker
    allowedOrientations: Orientation.All

    signal insertSnippet(string snippet)

    property string pendingSnippet: ""

    canAccept: pendingSnippet.length > 0

    // ── Element Data Model ──────────────────────────────────────────
    readonly property var allElements: [
        // Inline Formatting
        { name: "Heading 4",       category: "Inline",  snippet: "==== Heading\n",
          previewSnippet: "==== Section Title" },
        { name: "Bold",            category: "Inline",  snippet: "*text*",
          previewSnippet: "This is *bold* text" },
        { name: "Italic",          category: "Inline",  snippet: "_text_",
          previewSnippet: "This is _italic_ text" },
        { name: "Monospace",       category: "Inline",  snippet: "`code`",
          previewSnippet: "Use `printf()` here" },
        { name: "Strikethrough",   category: "Inline",  snippet: "~text~",
          previewSnippet: "This has ~deleted~ text" },
        { name: "Superscript",     category: "Inline",  snippet: "^text^",
          previewSnippet: "E = mc^2^" },
        { name: "Highlighted",     category: "Inline",  snippet: "#text#",
          previewSnippet: "This is #highlighted# text" },
        { name: "Footnote",        category: "Inline",  snippet: "footnote:[text]",
          previewSnippet: "Referencefootnote:[An important note.] here" },

        // Block Elements
        { name: "Heading 5",       category: "Block",   snippet: "===== Title\n",
          previewSnippet: "===== Deep Title" },
        { name: "Heading 6",       category: "Block",   snippet: "====== Title\n",
          previewSnippet: "====== Deepest Title" },
        { name: "Code block",      category: "Block",   snippet: "[source]\n----\ncode\n----\n",
          previewSnippet: "[source]\n----\nfn main() {\n    println!(\"hello\");\n}\n----" },
        { name: "Blockquote",      category: "Block",   snippet: "____\nQuote text\n____\n",
          previewSnippet: "[quote]\n____\nFamous words.\n____" },
        { name: "Verse",           category: "Block",   snippet: "[verse]\n____\nVerse text\n____\n",
          previewSnippet: "[verse]\n____\nThe road goes ever on.\n____" },
        { name: "Literal block",   category: "Block",   snippet: "....\nLiteral text\n....\n",
          previewSnippet: "....\n  Literal text here\n...." },
        { name: "Example block",   category: "Block",   snippet: "====\nExample content\n====\n",
          previewSnippet: ".Example\n====\nExample content\n====" },
        { name: "Open block",      category: "Block",   snippet: "--\nOpen content\n--\n",
          previewSnippet: "--\nOpen block content\n--" },
        { name: "Horizontal rule", category: "Block",   snippet: "---\n",
          previewSnippet: "---" },
        { name: "Page break",      category: "Block",   snippet: "<<<\n",
          previewSnippet: "<<<" },
        { name: "NOTE",            category: "Block",   snippet: "[NOTE]\n====\nNote content\n====\n",
          previewSnippet: "[NOTE]\n====\nNote text.\n====" },
        { name: "TIP",             category: "Block",   snippet: "[TIP]\n====\nTip content\n====\n",
          previewSnippet: "[TIP]\n====\nTip text.\n====" },
        { name: "WARNING",         category: "Block",   snippet: "[WARNING]\n====\nWarning content\n====\n",
          previewSnippet: "[WARNING]\n====\nWarning text.\n====" },
        { name: "CAUTION",         category: "Block",   snippet: "[CAUTION]\n====\nCaution content\n====\n",
          previewSnippet: "[CAUTION]\n====\nBe very careful.\n====" },
        { name: "IMPORTANT",       category: "Block",   snippet: "[IMPORTANT]\n====\nImportant content\n====\n",
          previewSnippet: "[IMPORTANT]\n====\nThis is critical.\n====" },
        { name: "Description list",category: "Block",   snippet: "Term:: Description\n",
          previewSnippet: "Term:: Description text" },

        // Tables & Media
        { name: "Table",           category: "Table",   snippet: "[cols=\"1,1\", options=\"header\"]\n|===\n| Header 1 | Header 2\n\n| Cell 1 | Cell 2\n|===\n",
          previewSnippet: "|===\n| Name | Age\n| Alice | 30\n|===" },
        { name: "Block image",     category: "Table",   snippet: "image::path.png[Alt text]\n",
          previewSnippet: "image::photo.jpg[A photo]" },
        { name: "Inline image",    category: "Table",   snippet: "image:path[Alt]",
          previewSnippet: "See image:icon.png[Icon,16] here" },
        { name: "Sidebar",         category: "Table",   snippet: "[sidebar]\n****\nSidebar text\n****\n",
          previewSnippet: "[sidebar]\n****\nSidebar text.\n****" },

        // Macros
        { name: "Icon",            category: "Macro",   snippet: "icon:name[]",
          previewSnippet: "Click icon:star[] to rate" },
        { name: "Keyboard",        category: "Macro",   snippet: "kbd:[Key]",
          previewSnippet: "Press kbd:[Ctrl+S] to save" },
        { name: "Button",          category: "Macro",   snippet: "btn:[Label]",
          previewSnippet: "Click btn:[Submit] to continue" },
        { name: "Menu",            category: "Macro",   snippet: "menu:File[Quit]",
          previewSnippet: "Use menu:File[Quit] to exit" },
        { name: "Cross-reference", category: "Macro",   snippet: "xref:page.adoc[Display]",
          previewSnippet: "See xref:other.adoc[Other Page]" },
        { name: "Stem / Math",     category: "Macro",   snippet: "stem:[formula]",
          previewSnippet: "The equation stem:[E = mc^2]" },
        { name: "Pass-through",    category: "Macro",   snippet: "pass:[text]",
          previewSnippet: "Use pass:[<b>raw HTML</b>] here" },

        // Document
        { name: "Index term",      category: "Document", snippet: "((term))",
          previewSnippet: "A ((concept)) in text" },
        { name: "Block comment",   category: "Document", snippet: "////\nComment text\n////\n",
          previewSnippet: "////\nThis is a block comment\n////" },
        { name: "Line comment",    category: "Document", snippet: "// Comment",
          previewSnippet: "// This is a line comment" },
        { name: "Document attr",   category: "Document", snippet: ":name: value\n",
          previewSnippet: ":author: Jane Doe" },
    ]

    readonly property var categories: ["Inline", "Block", "Table", "Macro", "Document"]

    // ── Search ──────────────────────────────────────────────────────
    property string searchQuery: ""

    readonly property var filteredElements: {
        var q = searchQuery.trim().toLowerCase()
        if (q.length === 0) return allElements
        var res = []
        for (var i = 0; i < allElements.length; i++) {
            var el = allElements[i]
            if (el.name.toLowerCase().indexOf(q) >= 0 ||
                el.snippet.toLowerCase().indexOf(q) >= 0 ||
                el.category.toLowerCase().indexOf(q) >= 0) {
                res.push(el)
            }
        }
        return res
    }

    // ── Preview Cache (loaded once from bridge) ─────────────────────
    property var previewCache: ({})

    Component.onCompleted: {
        if (typeof bridge !== "undefined" && bridge && bridge.render_element_previews) {
            try {
                var json = bridge.render_element_previews()
                previewCache = JSON.parse(json) || {}
            } catch (e) {
                previewCache = {}
            }
        }
    }

    function getPreviewHtml(previewSnippet) {
        return previewCache[previewSnippet] || ""
    }

    // ── Body ────────────────────────────────────────────────────────
    SilicaFlickable {
        anchors.fill: parent
        contentHeight: contentColumn.height + Theme.paddingLarge

        VerticalScrollDecorator {}

        Column {
            id: contentColumn
            width: parent.width
            spacing: Theme.paddingSmall

            DialogHeader {
                title: qsTr("Insert Element")
                acceptText: ""
                cancelText: qsTr("Close")
            }

            SearchField {
                id: searchField
                width: parent.width
                placeholderText: qsTr("Search %1 elements...").arg(elementPicker.allElements.length)
                onTextChanged: elementPicker.searchQuery = text
            }

            Repeater {
                model: elementPicker.categories

                delegate: Column {
                    width: contentColumn.width
                    property string category: modelData

                    property var categoryElements: {
                        var res = []
                        var filtered = elementPicker.filteredElements
                        for (var i = 0; i < filtered.length; i++) {
                            if (filtered[i].category === category) {
                                res.push(filtered[i])
                            }
                        }
                        return res
                    }

                    visible: categoryElements.length > 0
                    spacing: 0

                    SectionHeader {
                        text: category === "Inline" ? qsTr("Inline Formatting")
                             : category === "Block" ? qsTr("Block Elements")
                             : category === "Table" ? qsTr("Tables & Media")
                             : category === "Macro" ? qsTr("Macros")
                             : qsTr("Document")
                        visible: categoryElements.length > 0
                    }

                    Repeater {
                        model: categoryElements

                        delegate: BackgroundItem {
                            id: elementDelegate
                            width: contentColumn.width
                            height: Math.max(Theme.itemSizeMedium, elementRow.implicitHeight + Theme.paddingMedium * 2)

                            property var element: modelData

                            onClicked: {
                                elementPicker.pendingSnippet = element.snippet
                                elementPicker.insertSnippet(element.snippet)
                                elementPicker.accept()
                            }

                            Row {
                                id: elementRow
                                anchors {
                                    left: parent.left; right: parent.right
                                    leftMargin: Theme.horizontalPageMargin
                                    rightMargin: Theme.horizontalPageMargin
                                    verticalCenter: parent.verticalCenter
                                }
                                spacing: Theme.paddingMedium

                                Column {
                                    width: parent.width * 0.45 - Theme.paddingMedium
                                    anchors.verticalCenter: parent.verticalCenter
                                    spacing: 2

                                    Label {
                                        text: element.name
                                        font.pixelSize: Theme.fontSizeSmall
                                        font.bold: true
                                        color: elementDelegate.down ? Theme.highlightColor : Theme.primaryColor
                                        truncationMode: TruncationMode.Fade
                                        width: parent.width
                                    }

                                    Label {
                                        text: element.snippet
                                        font.pixelSize: Theme.fontSizeExtraSmall
                                        font.family: "monospace"
                                        color: Theme.secondaryColor
                                        truncationMode: TruncationMode.Fade
                                        width: parent.width
                                        maximumLineCount: 2
                                        wrapMode: Text.Wrap
                                    }
                                }

                                Rectangle {
                                    width: parent.width * 0.55 - Theme.paddingMedium
                                    height: previewLabel.implicitHeight + Theme.paddingSmall * 2
                                    anchors.verticalCenter: parent.verticalCenter
                                    radius: Theme.paddingSmall / 2
                                    color: Theme.rgba(Theme.highlightBackgroundColor, 0.08)
                                    border.color: Theme.rgba(Theme.primaryColor, 0.1)
                                    border.width: 1

                                    Label {
                                        id: previewLabel
                                        anchors {
                                            fill: parent
                                            margins: Theme.paddingSmall
                                        }
                                        textFormat: Text.RichText
                                        wrapMode: Text.Wrap
                                        font.pixelSize: Theme.fontSizeExtraSmall
                                        color: Theme.primaryColor
                                        linkColor: Theme.highlightColor
                                        maximumLineCount: 4
                                        text: {
                                            var html = elementPicker.getPreviewHtml(element.previewSnippet)
                                            if (html && html.length > 0) {
                                                return html.replace(/__LINK_COLOR__/g, Theme.highlightColor)
                                            }
                                            return "<pre style='color:" + Theme.secondaryColor + ";font-size:small;'>" +
                                                   element.snippet.replace(/</g, '&lt;').replace(/>/g, '&gt;') +
                                                   "</pre>"
                                        }
                                    }
                                }
                            }
                        }
                    }

                    Item { width: 1; height: Theme.paddingSmall }
                }
            }

            Label {
                width: parent.width - Theme.horizontalPageMargin * 2
                anchors.horizontalCenter: parent.horizontalCenter
                visible: elementPicker.filteredElements.length === 0
                text: qsTr("No elements match \"%1\"").arg(elementPicker.searchQuery)
                font.pixelSize: Theme.fontSizeSmall
                color: Theme.secondaryColor
                horizontalAlignment: Text.AlignHCenter
                topPadding: Theme.paddingLarge
                bottomPadding: Theme.paddingLarge
            }

            Item { width: 1; height: Theme.paddingLarge }
        }
    }

    onAccepted: {
        // insertSnippet already emitted on tap
    }
}
