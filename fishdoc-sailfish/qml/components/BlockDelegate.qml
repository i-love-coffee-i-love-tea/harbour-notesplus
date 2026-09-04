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
            onXrefActivated: function(target) {
                delegate.xrefActivated(target)
            }
        }
    }

    Component {
        id: paragraphComponent
        InlineText {
            spans: blockData.spans || []
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
                onLinkActivated: function(target) {
                    delegate.linkActivated(target)
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
                onLinkActivated: function(target) {
                    delegate.linkActivated(target)
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
            property int columnCount: (blockData.rows && blockData.rows[0]) ? blockData.rows[0].length : 1
            x: Theme.horizontalPageMargin
            width: parent.width - Theme.horizontalPageMargin * 2
            spacing: 1

            Repeater {
                model: blockData.rows || []

                delegate: Row {
                    property int rowIndex: index
                    spacing: 1
                    Repeater {
                        model: modelData || []

                        delegate: Rectangle {
                            width: tableComponent.width / Math.max(1, tableComponent.columnCount)
                            height: cellText.implicitHeight + Theme.paddingSmall
                            color: Theme.highlightBackgroundColor
                            opacity: 0.2

                            InlineText {
                                id: cellText
                                anchors.centerIn: parent
                                width: parent.width - Theme.paddingSmall * 2
                                spans: modelData || []
                                font.pixelSize: Theme.fontSizeSmall
                                font.bold: rowIndex === 0
                                x: 0
                                onXrefActivated: function(target) {
                                    delegate.xrefActivated(target)
                                }
                            }
                        }
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
