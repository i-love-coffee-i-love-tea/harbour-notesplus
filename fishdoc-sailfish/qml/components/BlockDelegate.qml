import QtQuick 2.6
import Sailfish.Silica 1.0
import "./delegates"
import "../js/BlockHtmlUtils.js" as BlockHtmlUtils

Item {
    id: delegate

    property var blockData: ({})
    property var localBlockData: blockData
    property var allBlocks: []
    property int blockIndex: -1
    property bool isTocCollapsed: false
    property bool interactive: true
    property int renderCounter: 0
    property string searchTerm: ""

    property bool isMatchedBySearch: {
        if (!searchTerm || searchTerm.length === 0 || !blockData) return false
        var q = searchTerm.toLowerCase().trim()
        var terms = q.split(/\s+/).filter(function(t) { return t.length > 0 })
        if (terms.length === 0) return false
        var textToSearch = ""
        if (blockData.raw_text) textToSearch += " " + blockData.raw_text
        if (blockData.raw) textToSearch += " " + blockData.raw
        if (blockData.lines) textToSearch += " " + blockData.lines.join(" ")
        if (blockData.term) textToSearch += " " + blockData.term
        if (blockData.title) textToSearch += " " + blockData.title
        var lower = textToSearch.toLowerCase()
        for (var i = 0; i < terms.length; i++) {
            if (lower.indexOf(terms[i]) !== -1) return true
        }
        return false
    }

    signal clicked(int index)
    signal editRequested(int index, string raw)
    signal xrefActivated(string target)
    signal checkboxToggled(int blockIndex, string itemPath)
    signal toggleToc(int blockIndex)
    signal jumpToBlock(int targetIndex)

    width: parent ? parent.width : Screen.width
    height: blockLoader.height

    onBlockDataChanged: {
        localBlockData = blockData
        renderCounter++
    }

    function toggleLocalCheckbox(subPath) {
        if (!localBlockData) return
        var copy = JSON.parse(JSON.stringify(localBlockData))
        if (!subPath || subPath === "") {
            copy.checked = !copy.checked
        } else {
            var parts = subPath.split(".")
            var cur = copy
            for (var i = 0; i < parts.length; i++) {
                var childIdx = parseInt(parts[i], 10)
                if (cur && cur.blocks && cur.blocks[childIdx]) {
                    if (i === parts.length - 1) {
                        cur.blocks[childIdx].checked = !cur.blocks[childIdx].checked
                    } else {
                        cur = cur.blocks[childIdx]
                    }
                }
            }
        }
        localBlockData = copy
        renderCounter++
    }

    Rectangle {
        anchors.fill: parent
        color: Theme.rgba(Theme.highlightBackgroundColor, 0.15)
        border.color: Theme.highlightColor
        border.width: 1
        radius: 4
        visible: delegate.isMatchedBySearch
        z: -1
    }

    MouseArea {
        anchors.fill: parent
        enabled: delegate.interactive && blockData && (blockData.type !== "toc")
        z: -1
        onClicked: {
            if (delegate.interactive) {
                delegate.clicked(delegate.blockIndex)
                delegate.editRequested(delegate.blockIndex, (delegate.localBlockData && delegate.localBlockData.raw !== undefined) ? delegate.localBlockData.raw : ((delegate.localBlockData && delegate.localBlockData.raw_text) ? delegate.localBlockData.raw_text : ""))
            }
        }
    }

    Loader {
        id: blockLoader
        width: parent.width
        sourceComponent: {
            if (!blockData || !blockData.type) return emptyComponent
            switch (blockData.type) {
                case "heading": return headingComponent
                case "paragraph": return paragraphComponent
                case "unordered_list_item":
                case "ordered_list_item":
                case "description_list_item":
                case "callout_list_item": return listComponent
                case "code_block":
                case "literal_block": return codeComponent
                case "blockquote":
                case "verse": return verseQuoteComponent
                case "table": return tableComponent
                case "admonition": return admonitionComponent
                case "sidebar":
                case "example":
                case "open": return sidebarComponent
                case "image": return imageComponent
                case "toc": return tocComponent
                case "horizontal_rule": return hrComponent
                case "page_break": return pageBreakComponent
                case "comment": return commentComponent
                case "empty_line": return emptyComponent
                default: return emptyComponent
            }
        }
    }

    Component {
        id: headingComponent
        HeadingDelegate {
            blockData: delegate.localBlockData
            onXrefActivated: function(target) { delegate.xrefActivated(target) }
        }
    }

    Component {
        id: paragraphComponent
        ParagraphDelegate {
            blockData: delegate.localBlockData
            onXrefActivated: function(target) { delegate.xrefActivated(target) }
        }
    }

    Component {
        id: listComponent
        ListDelegate {
            blockData: delegate.blockData
            localBlockData: delegate.localBlockData
            blockIndex: delegate.blockIndex
            renderCounter: delegate.renderCounter
            onXrefActivated: function(target) { delegate.xrefActivated(target) }
            onCheckboxToggled: function(idx, path) { delegate.checkboxToggled(idx, path) }
            onToggleLocalCheckbox: function(path) { delegate.toggleLocalCheckbox(path) }
        }
    }

    Component {
        id: codeComponent
        CodeDelegate {
            blockData: delegate.localBlockData
        }
    }

    Component {
        id: verseQuoteComponent
        VerseQuoteDelegate {
            blockData: delegate.localBlockData
            onXrefActivated: function(target) { delegate.xrefActivated(target) }
        }
    }

    Component {
        id: tableComponent
        TableDelegate {
            blockData: delegate.localBlockData
            onXrefActivated: function(target) { delegate.xrefActivated(target) }
        }
    }

    Component {
        id: admonitionComponent
        AdmonitionDelegate {
            blockData: delegate.localBlockData
            onXrefActivated: function(target) { delegate.xrefActivated(target) }
        }
    }

    Component {
        id: sidebarComponent
        SidebarDelegate {
            blockData: delegate.localBlockData
            blockIndex: delegate.blockIndex
            onXrefActivated: function(target) { delegate.xrefActivated(target) }
            onCheckboxToggled: function(idx, path) { delegate.checkboxToggled(idx, path) }
        }
    }

    Component {
        id: imageComponent
        ImageDelegate {
            blockData: delegate.localBlockData
        }
    }

    Component {
        id: tocComponent
        TocDelegate {
            blockData: delegate.blockData
            allBlocks: delegate.allBlocks
            blockIndex: delegate.blockIndex
            isTocCollapsed: delegate.isTocCollapsed
            interactive: delegate.interactive
            onToggleToc: function(idx) { delegate.toggleToc(idx) }
            onJumpToBlock: function(targetIdx) { delegate.jumpToBlock(targetIdx) }
        }
    }

    Component {
        id: hrComponent
        Separator {
            anchors.left: parent ? parent.left : undefined
            anchors.right: parent ? parent.right : undefined
            anchors.leftMargin: Theme.horizontalPageMargin
            anchors.rightMargin: Theme.horizontalPageMargin
            color: Theme.highlightColor
        }
    }

    Component {
        id: pageBreakComponent
        Item {
            anchors.left: parent ? parent.left : undefined
            anchors.right: parent ? parent.right : undefined
            anchors.leftMargin: Theme.horizontalPageMargin
            anchors.rightMargin: Theme.horizontalPageMargin
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
        id: commentComponent
        Label {
            text: "// " + (localBlockData ? (localBlockData.text || "") : "")
            anchors.left: parent ? parent.left : undefined
            anchors.right: parent ? parent.right : undefined
            anchors.leftMargin: Theme.horizontalPageMargin
            anchors.rightMargin: Theme.horizontalPageMargin
            font.italic: true
            font.family: "monospace"
            font.pixelSize: Math.round(Theme.fontSizeSmall * ((typeof app !== "undefined" && app && app.fontScale) ? app.fontScale : 1.0))
            color: Theme.rgba(Theme.secondaryColor, 0.6)
            wrapMode: Text.WrapAtWordBoundaryOrAnywhere
        }
    }

    Component {
        id: emptyComponent
        Item {
            height: Theme.paddingSmall
            width: parent ? parent.width : Screen.width
        }
    }
}
