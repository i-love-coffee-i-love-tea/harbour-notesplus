import QtQuick 2.6
import Sailfish.Silica 1.0

Item {
    id: delegate
    property var blockData: ({})
    signal xrefActivated(string target)

    height: contentItem.height + Theme.paddingSmall

    Loader {
        id: contentItem
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
                case "admonition": return admonitionComponent
                case "horizontal_rule": return hrComponent
                case "empty_line": return emptyComponent
                default: return paragraphComponent
            }
        }
    }

    Component {
        id: headingComponent
        InlineText {
            property int level: blockData.level || 1
            spans: blockData.spans || []
            color: Theme.highlightColor
            font.pixelSize: {
                switch (level) {
                    case 1: return Theme.fontSizeExtraLarge
                    case 2: return Theme.fontSizeLarge
                    case 3: return Theme.fontSizeMedium
                    default: return Theme.fontSizeSmall
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
            onXrefActivated: function(target) {
                delegate.xrefActivated(target)
            }
        }
    }

    Component {
        id: orderedListComponent
        Row {
            spacing: Theme.paddingSmall
            x: Theme.horizontalPageMargin + (blockData.level || 0) * Theme.paddingLarge

            Label {
                text: blockData.marker || "1."
                color: Theme.secondaryColor
                font.pixelSize: Theme.fontSizeSmall
            }

            InlineText {
                spans: blockData.spans || []
                width: parent.parent.width - Theme.horizontalPageMargin * 2 - (blockData.level || 0) * Theme.paddingLarge - Theme.paddingSmall
                onXrefActivated: function(target) {
                    delegate.xrefActivated(target)
                }
            }
        }
    }

    Component {
        id: unorderedListComponent
        Row {
            spacing: Theme.paddingSmall
            x: Theme.horizontalPageMargin + (blockData.level || 0) * Theme.paddingLarge

            Label {
                text: blockData.marker || "-"
                color: Theme.secondaryColor
                font.pixelSize: Theme.fontSizeSmall
            }

            InlineText {
                spans: blockData.spans || []
                width: parent.parent.width - Theme.horizontalPageMargin * 2 - (blockData.level || 0) * Theme.paddingLarge - Theme.paddingSmall
                onXrefActivated: function(target) {
                    delegate.xrefActivated(target)
                }
            }
        }
    }

    Component {
        id: codeBlockComponent
        Rectangle {
            color: Theme.highlightBackgroundColor
            opacity: 0.3
            radius: 4
            height: codeLabel.height + Theme.paddingSmall * 2
            x: Theme.horizontalPageMargin
            width: parent.width - Theme.horizontalPageMargin * 2

            Label {
                id: codeLabel
                anchors {
                    left: parent.left
                    right: parent.right
                    margins: Theme.paddingSmall
                    verticalCenter: parent.verticalCenter
                }
                text: (blockData.lines || []).join("\n")
                font.family: "monospace"
                font.pixelSize: Theme.fontSizeSmall
                color: Theme.primaryColor
                wrapMode: Text.Wrap
            }
        }
    }

    Component {
        id: literalBlockComponent
        Rectangle {
            color: Theme.highlightBackgroundColor
            opacity: 0.2
            radius: 4
            height: litLabel.height + Theme.paddingSmall * 2
            x: Theme.horizontalPageMargin
            width: parent.width - Theme.horizontalPageMargin * 2

            Label {
                id: litLabel
                anchors {
                    left: parent.left
                    right: parent.right
                    margins: Theme.paddingSmall
                    verticalCenter: parent.verticalCenter
                }
                text: (blockData.lines || []).join("\n")
                font.family: "monospace"
                font.pixelSize: Theme.fontSizeSmall
                color: Theme.primaryColor
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

            Grid {
                id: tableGrid
                property int columnCount: (blockData.rows && blockData.rows[0]) ? blockData.rows[0].length : 1
                property real cellWidth: (parent.width - (columnCount - 1) * spacing) / Math.max(1, columnCount)
                columns: columnCount
                spacing: 1

                Repeater {
                    id: tableRepeater
                    model: {
                        var rows = blockData.rows || []
                        var flat = []
                        for (var r = 0; r < rows.length; r++) {
                            var row = rows[r]
                            for (var c = 0; c < row.length; c++) {
                                flat.push({ spans: row[c], header: r === 0 })
                            }
                        }
                        return flat
                    }

                    delegate: Rectangle {
                        width: tableGrid.cellWidth
                        height: Math.max(48, cellText.paintedHeight + 16)
                        color: "transparent"
                        clip: true

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
                            font.pixelSize: Theme.fontSizeSmall
                            font.bold: modelData.header
                            color: Theme.primaryColor
                            text: {
                                var spans = modelData.spans || []
                                if (typeof spans === "string") return spans
                                var parts = []
                                for (var i = 0; i < spans.length; i++) {
                                    var s = spans[i]
                                    if (s) parts.push(s.value || s.display || s.target || "")
                                }
                                return parts.join("") || " "
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
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.leftMargin: Theme.horizontalPageMargin
            anchors.rightMargin: Theme.horizontalPageMargin
            height: admonitionRect.height

            Rectangle {
                id: admonitionRect
                anchors.left: parent.left
                anchors.right: parent.right
                color: "transparent"
                border.color: Theme.highlightColor
                border.width: 3
                radius: 4
                height: admonitionColumn.height + Theme.paddingMedium * 2

                Column {
                    id: admonitionColumn
                    anchors {
                        left: parent.left
                        right: parent.right
                        margins: Theme.paddingMedium
                        top: parent.top
                        topMargin: Theme.paddingMedium
                    }
                    spacing: Theme.paddingSmall

                    Label {
                        text: kind
                        font.bold: true
                        font.pixelSize: Theme.fontSizeSmall
                        color: admonitionRect.border.color
                    }

                    InlineText {
                        spans: blockData.spans || []
                        width: parent.width
                        font.pixelSize: Theme.fontSizeSmall
                    }
                }
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
