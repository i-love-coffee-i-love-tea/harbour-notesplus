import QtQuick 2.6
import Sailfish.Silica 1.0
import "./delegates"

Item {
    id: delegate

    property var blockData: ({})
    property var localBlockData: blockData
    property int blockIndex: -1
    property bool isTocCollapsed: false
    property bool interactive: true
    property int renderCounter: 0
    property string searchTerm: ""
    property bool isEditing: false
    property string editingRawText: ""

    property bool isMatchedBySearch: (interactive && searchTerm && searchTerm.length > 0 && blockData) ? checkSearchMatch() : false

    function checkSearchMatch() {
        if (!blockData) return false
        var q = searchTerm.toLowerCase().trim()
        if (q.length === 0) return false
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
    signal textModified(int index, string text)
    signal saveRequested(int index, string raw)
    signal cancelEditRequested()
    signal xrefActivated(string target)
    signal checkboxToggled(int blockIndex, string itemPath)
    signal toggleToc(int blockIndex)
    signal jumpToBlock(int targetIndex)

    width: parent ? parent.width : Screen.width
    height: isEditing ? (inlineEditorContainer.height + Theme.paddingSmall) : blockLoader.height

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
        visible: delegate.interactive && delegate.isMatchedBySearch
        z: -1
    }

    MouseArea {
        anchors.fill: parent
        enabled: delegate.interactive && !delegate.isEditing && blockData && (blockData.type !== "toc")
        visible: delegate.interactive && !delegate.isEditing
        onClicked: {
            if (delegate.interactive && !delegate.isEditing) {
                var raw = ""
                if (delegate.localBlockData && delegate.localBlockData.raw !== undefined) {
                    raw = delegate.localBlockData.raw
                } else if (delegate.blockData && delegate.blockData.raw !== undefined) {
                    raw = delegate.blockData.raw
                } else if (delegate.localBlockData && delegate.localBlockData.raw_text) {
                    raw = delegate.localBlockData.raw_text
                } else if (delegate.blockData && delegate.blockData.raw_text) {
                    raw = delegate.blockData.raw_text
                }
                delegate.clicked(delegate.blockIndex)
                delegate.editRequested(delegate.blockIndex, raw)
            }
        }
    }

    Loader {
        id: blockLoader
        width: parent.width
        visible: !delegate.isEditing
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
            searchTerm: delegate.searchTerm
            onXrefActivated: function(target) { delegate.xrefActivated(target) }
        }
    }

    Component {
        id: paragraphComponent
        ParagraphDelegate {
            blockData: delegate.localBlockData
            searchTerm: delegate.searchTerm
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
            searchTerm: delegate.searchTerm
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
            font.pixelSize: app.scaledFontSize(Theme.fontSizeSmall)
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

    Item {
        id: inlineEditorContainer
        width: parent ? parent.width : Screen.width
        height: isEditing ? (inlineTextArea.implicitHeight + Theme.paddingMedium) : 0
        visible: isEditing

        // Subtle vertical accent line on the left to indicate the active in-place block
        Rectangle {
            anchors.left: parent.left
            anchors.leftMargin: Theme.horizontalPageMargin / 2
            anchors.top: parent.top
            anchors.topMargin: Theme.paddingSmall
            anchors.bottom: parent.bottom
            anchors.bottomMargin: Theme.paddingSmall
            width: 2
            color: Theme.highlightColor
            opacity: 0.8
            radius: 1
        }

        TextArea {
            id: inlineTextArea
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.leftMargin: Theme.horizontalPageMargin
            anchors.rightMargin: Theme.horizontalPageMargin + Theme.itemSizeMedium
            anchors.verticalCenter: parent.verticalCenter
            text: delegate.editingRawText
            placeholderText: qsTr("Edit block...")
            font.pixelSize: app.scaledFontSize(Theme.fontSizeMedium)
            color: Theme.primaryColor
            background: null
            onTextChanged: {
                if (delegate.isEditing) {
                    delegate.textModified(delegate.blockIndex, text)
                }
            }
        }
    }

    onIsEditingChanged: {
        if (isEditing) {
            inlineTextArea.text = editingRawText
            inlineTextArea.forceActiveFocus()
        }
    }

    onEditingRawTextChanged: {
        if (isEditing) {
            inlineTextArea.text = editingRawText
        }
    }
}
