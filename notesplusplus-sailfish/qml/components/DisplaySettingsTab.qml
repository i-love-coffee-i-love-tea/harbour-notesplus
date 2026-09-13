import QtQuick 2.6
import Sailfish.Silica 1.0
import "."

Column {
    id: displaySettingsTab
    width: parent.width
    spacing: Theme.paddingMedium

    SectionHeader {
        text: qsTr("Font & Typography")
    }

    ComboBox {
        width: parent.width
        label: qsTr("Font Family")
        currentIndex: {
            switch (app.docFontFamily) {
                case "serif": return 1
                case "sans-serif": return 2
                case "monospace": return 3
                default: return 0
            }
        }
        menu: ContextMenu {
            MenuItem { text: qsTr("Sailfish Default") }
            MenuItem { text: qsTr("Serif") }
            MenuItem { text: qsTr("Sans-Serif") }
            MenuItem { text: qsTr("Monospace") }
        }
        onCurrentIndexChanged: {
            switch (currentIndex) {
                case 1: app.setFontFamily("serif"); break
                case 2: app.setFontFamily("sans-serif"); break
                case 3: app.setFontFamily("monospace"); break
                default: app.setFontFamily(""); break
            }
        }
    }

    Slider {
        width: parent.width
        minimumValue: 0.8
        maximumValue: 1.5
        stepSize: 0.1
        value: app.fontScale
        label: qsTr("Document Font Size")
        valueText: {
            var pct = Math.round(value * 100)
            if (pct === 100) return qsTr("100% (Default)")
            return pct + "%"
        }
        onSliderValueChanged: {
            app.setFontScale(Math.round(value * 10) / 10)
        }
    }

    Slider {
        width: parent.width
        minimumValue: 0.8
        maximumValue: 1.5
        stepSize: 0.1
        value: app.codeFontScale
        label: qsTr("Code Font Size")
        valueText: {
            var pct = Math.round(value * 100)
            if (pct === 100) return qsTr("100% (Default)")
            return pct + "%"
        }
        onSliderValueChanged: {
            app.setCodeFontScale(Math.round(value * 10) / 10)
        }
    }

    SectionHeader {
        text: qsTr("Document View")
    }

    Slider {
        width: parent.width
        minimumValue: 0
        maximumValue: 20
        stepSize: 1
        value: (typeof app !== "undefined" && app && app.tocCollapseThreshold !== undefined) ? app.tocCollapseThreshold : 5
        label: qsTr("Collapse Table of Contents")
        valueText: {
            var v = Math.round(value)
            if (v === 0) return qsTr("Always (0+ headings)")
            if (v >= 20) return qsTr("Never (Always expanded)")
            return qsTr("When more than %1 headings").arg(v)
        }
        onSliderValueChanged: {
            if (typeof app !== "undefined" && app && app.setTocCollapseThreshold) {
                app.setTocCollapseThreshold(Math.round(value))
            }
        }
    }

    Slider {
        width: parent.width
        minimumValue: 0.3
        maximumValue: 0.8
        stepSize: 0.02
        value: (typeof app !== "undefined" && app && app.previewScale !== undefined) ? app.previewScale : 0.52
        label: qsTr("Preview Scaling Factor")
        valueText: Math.round(value * 100) + "%"
        onSliderValueChanged: {
            if (typeof app !== "undefined" && app && app.setPreviewScale) {
                app.setPreviewScale(Math.round(value * 100) / 100)
            }
        }
    }

    TextSwitch {
        width: parent.width
        text: qsTr("Strip Comments")
        description: qsTr("Standard AsciiDoc behavior drops // comments from rendered documents")
        checked: (typeof app !== "undefined" && app && app.dropComments !== undefined) ? app.dropComments : true
        onCheckedChanged: {
            if (typeof app !== "undefined" && app && app.setDropComments) {
                app.setDropComments(checked)
            }
        }
    }

    TextSwitch {
        width: parent.width
        text: qsTr("Allow External Images")
        description: qsTr("Load images from remote HTTP and HTTPS web addresses")
        checked: (typeof app !== "undefined" && app && app.allowExternalImages !== undefined) ? app.allowExternalImages : true
        onCheckedChanged: {
            if (typeof app !== "undefined" && app && app.setAllowExternalImages) {
                app.setAllowExternalImages(checked)
            }
        }
    }

    TextSwitch {
        width: parent.width
        text: qsTr("Enable Daily Journal")
        description: qsTr("Show daily journal entries and quick capture on the main page")
        checked: (typeof app !== "undefined" && app && app.journalEnabled !== undefined) ? app.journalEnabled : true
        onCheckedChanged: {
            if (typeof app !== "undefined" && app && app.setJournalEnabled) {
                app.setJournalEnabled(checked)
            }
        }
    }

    SectionHeader {
        text: qsTr("Note Groups")
    }

    Slider {
        width: parent.width
        minimumValue: 1
        maximumValue: 5
        stepSize: 1
        value: (typeof app !== "undefined" && app && app.groupDisplayDepth !== undefined) ? app.groupDisplayDepth : 2
        label: qsTr("Group Display Depth")
        valueText: qsTr("%1 level(s)").arg(Math.round(value))
        onSliderValueChanged: {
            if (typeof app !== "undefined" && app && app.setGroupDisplayDepth) {
                app.setGroupDisplayDepth(Math.round(value))
            }
        }
    }

    BackgroundItem {
        width: parent.width
        height: Theme.itemSizeMedium
        onClicked: {
            var result = bridge.rebuild_index()
            rebuildResultLabel.text = result
        }
        Row {
            anchors.fill: parent
            anchors.leftMargin: Theme.horizontalPageMargin
            anchors.rightMargin: Theme.horizontalPageMargin
            spacing: Theme.paddingMedium
            Icon {
                anchors.verticalCenter: parent.verticalCenter
                source: "image://theme/icon-m-refresh"
                color: parent.parent.highlighted ? Theme.highlightColor : Theme.primaryColor
            }
            Column {
                anchors.verticalCenter: parent.verticalCenter
                Label {
                    text: qsTr("Rebuild Index")
                    color: parent.parent.parent.highlighted ? Theme.highlightColor : Theme.primaryColor
                }
                Label {
                    id: rebuildResultLabel
                    text: qsTr("Rescan all notes from disk and rebuild the search index")
                    font.pixelSize: Theme.fontSizeExtraSmall
                    color: Theme.secondaryColor
                }
            }
        }
    }

    SectionHeader {
        text: qsTr("Live Preview")
    }

    Rectangle {
        width: parent.width - Theme.horizontalPageMargin * 2
        x: Theme.horizontalPageMargin
        height: previewCol.height + Theme.paddingMedium * 2
        color: Theme.rgba(Theme.highlightBackgroundColor, 0.08)
        border.color: Theme.rgba(Theme.primaryColor, 0.15)
        border.width: 1
        radius: Theme.paddingSmall

        Column {
            id: previewCol
            anchors {
                left: parent.left
                right: parent.right
                top: parent.top
                margins: Theme.paddingMedium
            }
            spacing: Theme.paddingSmall

            Label {
                width: parent.width
                text: "Sample Document Heading"
                color: Theme.highlightColor
                font.bold: true
                font.family: app.resolvedFontFamily()
                font.pixelSize: app.scaledFontSize(Theme.fontSizeLarge)
                wrapMode: Text.Wrap
            }

            Label {
                width: parent.width
                text: "This is a live preview of body text with <b>bold</b>, <i>italic</i>, and <code style='background:#18181c;color:#f2f2f7;padding:1px 4px;border-radius:3px;font-family:monospace;'>code spans</code>."
                textFormat: Text.RichText
                color: Theme.primaryColor
                font.family: app.resolvedFontFamily()
                font.pixelSize: app.scaledFontSize(Theme.fontSizeMedium)
                wrapMode: Text.Wrap
            }

            Rectangle {
                width: parent.width
                color: "#18181c"
                border.color: Theme.rgba(Theme.primaryColor, 0.2)
                border.width: 1
                radius: Theme.paddingSmall
                height: previewCodeCol.height + Theme.paddingMedium * 2

                Column {
                    id: previewCodeCol
                    anchors {
                        left: parent.left
                        right: parent.right
                        top: parent.top
                        margins: Theme.paddingMedium
                    }
                    spacing: Theme.paddingSmall / 2

                    Label {
                        text: "RUST"
                        font.family: "monospace"
                        font.pixelSize: Math.round(Theme.fontSizeExtraSmall * app.codeFontScale)
                        color: Theme.rgba("#f2f2f7", 0.6)
                    }

                    Label {
                        width: parent.width
                        text: "fn main() {\n    println!(\"Hello Notes++!\");\n}"
                        font.family: "monospace"
                        font.pixelSize: Math.round(Theme.fontSizeSmall * app.codeFontScale)
                        color: "#f2f2f7"
                        wrapMode: Text.Wrap
                    }
                }
            }

            Label {
                text: "Card Miniature"
                color: Theme.highlightColor
                font.pixelSize: Theme.fontSizeExtraSmall
                font.bold: true
            }

            MiniDocPreview {
                width: parent.width
                previewHeight: width * 0.6
                previewBlocksJson: JSON.stringify([
                    { type: "heading", level: 1, spans: [{ type: "text", value: "Document Title" }] },
                    { type: "toc", headings: [{ level: 1, text: "Document Title" }, { level: 2, text: "Getting Started" }, { level: 2, text: "Usage Guide" }] },
                    { type: "paragraph", spans: [{ type: "text", value: "Preview demonstrates scaling and TOC settings." }] },
                    { type: "unordered_list_item", checked: true, blocks: [{ type: "paragraph", spans: [{ type: "text", value: "Configurable scaling" }] }] }
                ])
            }
        }
    }
}
