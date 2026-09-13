import QtQuick 2.6
import Sailfish.Silica 1.0
import "../components"
import "../js/BlockHtmlUtils.js" as BlockHtmlUtils

Page {
    id: pageView
    allowedOrientations: Orientation.All

    property string pageName: bridge.current_page_name
    property string searchTerm: ""
    property string findInPageTerm: ""
    property bool showFindBar: false
    property int targetBlockIndex: -1
    property bool hasScrolledToSearchTerm: false
    property int editingBlockIndex: -1
    property int editingBlockCount: 1
    property string editingRawText: ""
    property string editingCurrentText: ""
    property bool isAddingNewBlock: false
    property string newBlockText: ""

    property var collapsedTocBlocks: ({})

    function isTocCollapsed(idx, entryCount) {
        if (collapsedTocBlocks[idx] !== undefined) {
            return !!collapsedTocBlocks[idx]
        }
        var threshold = (typeof app !== "undefined" && app && app.tocCollapseThreshold !== undefined) ? app.tocCollapseThreshold : 5
        if (threshold === 0) return true
        if (threshold >= 20) return false
        var count = entryCount !== undefined ? entryCount : 0
        if (count === 0 && parsedBlocks && parsedBlocks[idx] && parsedBlocks[idx].headings) {
            count = parsedBlocks[idx].headings.length
        }
        return count > threshold
    }

    function toggleToc(idx, entryCount) {
        var copy = {}
        for (var k in collapsedTocBlocks) {
            copy[k] = collapsedTocBlocks[k]
        }
        var current = isTocCollapsed(idx, entryCount)
        copy[idx] = !current
        collapsedTocBlocks = copy
    }

    function findBlockIndexForSearch(query) {
        if (!query || query.length === 0 || !parsedBlocks || parsedBlocks.length === 0) return -1
        var q = query.toLowerCase().trim()
        var terms = q.split(/\s+/).filter(function(t) { return t.length > 0 })
        if (terms.length === 0) return -1

        for (var i = 0; i < parsedBlocks.length; i++) {
            var b = parsedBlocks[i]
            var textToSearch = ""
            if (b.raw_text) textToSearch += " " + b.raw_text
            if (b.raw) textToSearch += " " + b.raw
            if (b.lines) textToSearch += " " + b.lines.join(" ")
            if (b.term) textToSearch += " " + b.term
            if (b.title) textToSearch += " " + b.title
            var lower = textToSearch.toLowerCase()

            for (var t = 0; t < terms.length; t++) {
                if (lower.indexOf(terms[t]) !== -1) {
                    return i
                }
            }
        }
        return -1
    }

    function scrollToSearchTarget() {
        if (hasScrolledToSearchTerm) return
        var targetIdx = targetBlockIndex
        if (targetIdx < 0 && searchTerm.length > 0) {
            targetIdx = findBlockIndexForSearch(searchTerm)
        }
        if (targetIdx >= 0 && targetIdx < (parsedBlocks ? parsedBlocks.length : 0)) {
            hasScrolledToSearchTerm = true
            scrollTimer.targetIdx = targetIdx
            scrollTimer.start()
        }
    }

    Timer {
        id: scrollTimer
        interval: 100
        property int targetIdx: -1
        onTriggered: {
            if (targetIdx >= 0) {
                listView.positionViewAtIndex(targetIdx, ListView.Beginning)
            }
        }
    }

    onParsedBlocksChanged: {
        if (searchTerm.length > 0 && !hasScrolledToSearchTerm) {
            scrollToSearchTarget()
        }
    }

    property var parsedBlocks: {
        var raw = bridge.blocks_version >= 0 ? bridge.current_blocks : []
        if (!raw) return []
        var list = []
        for (var i = 0; i < raw.length; i++) {
            try {
                list.push(JSON.parse(raw[i]))
            } catch (e) {
                list.push({})
            }
        }
        return list
    }

    function toggleParsedBlockCheckbox(idx, itemPath) {
        if (!parsedBlocks || idx < 0 || idx >= parsedBlocks.length) return
        var target = parsedBlocks[idx]
        if (itemPath && itemPath.length > 0) {
            var parts = itemPath.split(".")
            for (var p = 0; p < parts.length; p++) {
                var childIdx = parseInt(parts[p], 10)
                if (target && target.blocks && target.blocks[childIdx]) {
                    target = target.blocks[childIdx]
                } else {
                    return
                }
            }
        }
        if (target && target.checked !== undefined && target.checked !== null) {
            target.checked = !target.checked
        }
    }

    SilicaListView {
        id: listView
        anchors.fill: parent
        clip: true
        cacheBuffer: 3500
        model: pageView.parsedBlocks

        PullDownMenu {
            MenuItem {
                text: qsTr("Find in Page")
                onClicked: {
                    pageView.showFindBar = !pageView.showFindBar
                    if (pageView.showFindBar) {
                        findField.forceActiveFocus()
                    } else {
                        pageView.findInPageTerm = ""
                    }
                }
            }
            MenuItem {
                text: qsTr("Ask AI Assistant")
                visible: (typeof app !== "undefined" && app && app.aiEnabled !== undefined) ? app.aiEnabled : true
                onClicked: {
                    var content = bridge.get_page_source(pageName)
                    var fname = pageName.indexOf(".adoc") >= 0 ? pageName : (pageName + ".adoc")
                    app.openAssistant(fname, content)
                }
            }
            MenuItem {
                text: qsTr("Edit Source")
                onClicked: {
                    var editor = pageStack.push(Qt.resolvedUrl("PageSourceEditor.qml"), {
                        pageName: pageName
                    })
                    editor.accepted.connect(function() {
                        bridge.load_page(pageName)
                    })
                }
            }
            MenuItem {
                text: qsTr("Settings")
                onClicked: {
                    pageStack.push(Qt.resolvedUrl("SettingsPage.qml"))
                }
            }
            MenuItem {
                text: qsTr("Copy Page URL")
                onClicked: {
                    var url = bridge.open_in_browser(pageName)
                    if (url) {
                        Clipboard.text = url
                        remorsePopup.execute(qsTr("Copied: ") + url, function() {}, 3000)
                    }
                }
            }
            MenuItem {
                text: qsTr("Export to HTML5")
                onClicked: {
                    var path = bridge.export_html(pageName)
                    if (path) {
                        remorsePopup.execute(qsTr("Exported to ") + path, function() {})
                    }
                }
            }
            MenuItem {
                text: qsTr("Move to Group...")
                visible: !bridge.is_journal_page && pageName !== "Journal" && pageName !== "journal"
                onClicked: {
                    var fullPath = bridge.current_page_group_path.length > 0 ? bridge.current_page_group_path + "/" + pageName : pageName
                    var dialog = pageStack.push(Qt.resolvedUrl("MovePageDialog.qml"), {
                        pageFullPath: fullPath,
                        pageTitle: pageName,
                        currentGroup: bridge.current_page_group_path
                    })
                    dialog.accepted.connect(function() {
                        var target = dialog.targetGroup
                        remorsePopup.execute(qsTr("Moving to %1").arg(target.length > 0 ? target : qsTr("Root")), function() {
                            bridge.move_page_to_group(fullPath, target)
                        })
                    })
                }
            }
            MenuItem {
                text: qsTr("Delete Page")
                visible: !bridge.is_journal_page && pageName !== "Journal" && pageName !== "journal"
                onClicked: {
                    remorsePopup.execute(qsTr("Deleting page"), function() {
                        bridge.delete_page(pageName)
                        pageStack.pop()
                    })
                }
            }
        }

        header: Column {
            width: listView.width

            PageHeader {
                title: pageName
            }

            Label {
                id: breadcrumbLabel
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.leftMargin: Theme.horizontalPageMargin
                anchors.rightMargin: Theme.horizontalPageMargin
                visible: bridge.current_page_group_path.length > 0 && !bridge.is_journal_page
                text: bridge.current_page_group_path.split("/").join(" › ")
                color: Theme.highlightColor
                font.pixelSize: Theme.fontSizeExtraSmall
                truncationMode: TruncationMode.Fade
            }

            // Find-in-Page search bar
            Row {
                width: parent.width - Theme.horizontalPageMargin * 2
                anchors.horizontalCenter: parent.horizontalCenter
                spacing: Theme.paddingSmall
                visible: pageView.showFindBar

                SearchField {
                    id: findField
                    width: parent.width - closeFindBtn.width - Theme.paddingSmall
                    placeholderText: qsTr("Find in page...")
                    onTextChanged: {
                        pageView.findInPageTerm = text
                        if (text.length > 0) {
                            var idx = pageView.findBlockIndexForSearch(text)
                            if (idx >= 0) {
                                scrollTimer.targetIdx = idx
                                scrollTimer.start()
                            }
                        }
                    }
                    EnterKey.onClicked: {
                        // Jump to next match
                        if (pageView.findInPageTerm.length > 0) {
                            var idx = pageView.findBlockIndexForSearch(pageView.findInPageTerm)
                            if (idx >= 0) {
                                listView.positionViewAtIndex(idx, ListView.Beginning)
                            }
                        }
                    }
                }

                IconButton {
                    id: closeFindBtn
                    icon.source: "image://theme/icon-m-close"
                    anchors.verticalCenter: parent.verticalCenter
                    onClicked: {
                        pageView.showFindBar = false
                        pageView.findInPageTerm = ""
                        findField.text = ""
                    }
                }
            }
        }

        footer: Item {
            id: listFooter
            width: listView.width
            height: pageView.isAddingNewBlock ? (newBlockEditor.height + Theme.paddingLarge * 2) : (Theme.itemSizeExtraLarge * 2)

            MouseArea {
                anchors.fill: parent
                enabled: !pageView.isAddingNewBlock && pageView.editingBlockIndex < 0
                onClicked: {
                    pageView.startAddingNewBlock()
                }
            }

            Item {
                id: newBlockEditor
                width: parent.width
                height: pageView.isAddingNewBlock ? (inlineNewTextArea.implicitHeight + Theme.paddingMedium) : 0
                visible: pageView.isAddingNewBlock
                anchors.top: parent.top
                anchors.topMargin: Theme.paddingSmall

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
                    id: inlineNewTextArea
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.leftMargin: Theme.horizontalPageMargin
                    anchors.rightMargin: Theme.horizontalPageMargin + Theme.itemSizeMedium
                    anchors.verticalCenter: parent.verticalCenter
                    text: pageView.newBlockText
                    placeholderText: qsTr("Type text, task (* [ ]), or heading...")
                    font.pixelSize: app.scaledFontSize(Theme.fontSizeMedium)
                    color: Theme.primaryColor
                    background: null
                    onTextChanged: {
                        if (pageView.isAddingNewBlock) {
                            pageView.newBlockText = text
                        }
                    }
                }
            }
        }

        delegate: BlockDelegate {
            width: listView.width
            blockData: modelData || ({})
            blockIndex: index
            searchTerm: pageView.findInPageTerm || pageView.searchTerm
            editingRawText: (pageView.editingBlockIndex === index) ? pageView.editingRawText : ""
            isEditing: pageView.editingBlockIndex === index
            isTocCollapsed: pageView.isTocCollapsed(index, (blockData && blockData.headings) ? blockData.headings.length : 0)
            onToggleToc: function(idx) {
                pageView.toggleToc(idx, (blockData && blockData.headings) ? blockData.headings.length : 0)
            }
            onJumpToBlock: function(targetIndex) {
                listView.positionViewAtIndex(targetIndex, ListView.Beginning)
            }
            onXrefActivated: function(target) {
                bridge.navigate_to_page(target)
            }
            onCheckboxToggled: function(idx, itemPath) {
                pageView.toggleParsedBlockCheckbox(idx, itemPath)
                bridge.toggle_checkbox(idx, itemPath)
            }
            onEditRequested: function(idx, raw) {
                if (pageView.isAddingNewBlock) {
                    pageView.saveNewBlock()
                }
                var range = pageView.findConsecutiveListRange(idx)
                var actualRaw = ""
                if (range.count > 1) {
                    var parts = []
                    for (var r = range.start; r < range.start + range.count; r++) {
                        var rb = pageView.parsedBlocks[r]
                        if (rb) parts.push(rb.raw || rb.raw_text || "")
                    }
                    actualRaw = parts.join("\n")
                } else {
                    actualRaw = raw
                    if ((!actualRaw || actualRaw.length === 0) && pageView.parsedBlocks && pageView.parsedBlocks[idx]) {
                        var b = pageView.parsedBlocks[idx]
                        actualRaw = (b.raw !== undefined) ? b.raw : (b.raw_text || "")
                    }
                }
                pageView.editingRawText = actualRaw
                pageView.editingCurrentText = actualRaw
                pageView.editingBlockIndex = range.start
                pageView.editingBlockCount = range.count
            }
            onTextModified: function(idx, newText) {
                if (pageView.editingBlockIndex === idx) {
                    pageView.editingCurrentText = newText
                }
            }
            onSaveRequested: function(idx, newRaw) {
                bridge.save_block(idx, newRaw)
                pageView.editingBlockIndex = -1
                pageView.editingRawText = ""
                pageView.editingCurrentText = ""
            }
            onCancelEditRequested: function() {
                pageView.cancelCurrentEditing()
            }
        }

        RemorsePopup { id: remorsePopup }
    }

    // Stationary floating action sidebar for in-place editing (transparent, non-moving)
    InPlaceEditSidebar {
        id: inPlaceSidebar
        anchors.right: parent.right
        anchors.rightMargin: Theme.paddingMedium
        anchors.verticalCenter: parent.verticalCenter
        visible: pageView.editingBlockIndex >= 0 || pageView.isAddingNewBlock
        onAccepted: {
            if (pageView.isAddingNewBlock) {
                pageView.saveNewBlock()
            } else {
                pageView.saveCurrentEditingBlock()
            }
        }
        onCanceled: {
            if (pageView.isAddingNewBlock) {
                pageView.cancelNewBlock()
            } else {
                pageView.cancelCurrentEditing()
            }
        }
        onPrefixRequested: function(prefix, multiLine) {
            if (pageView.isAddingNewBlock) {
                pageView.applyNewBlockPrefix(prefix, multiLine)
            } else {
                pageView.applyBlockPrefix(prefix, multiLine)
            }
        }
        onLinkRequested: {
            var dialog = pageStack.push(Qt.resolvedUrl("PageLinkDialog.qml"), {
                selectedText: ""
            })
            dialog.accepted.connect(function() {
                var link = dialog.formattedLink
                if (!link) return
                if (pageView.isAddingNewBlock) {
                    pageView.newBlockText = (pageView.newBlockText && pageView.newBlockText.length > 0 ? pageView.newBlockText + " " : "") + link
                } else if (pageView.editingBlockIndex >= 0) {
                    var cur = pageView.editingCurrentText || ""
                    var updated = (cur.length > 0 ? cur + " " : "") + link
                    pageView.editingRawText = updated
                    pageView.editingCurrentText = updated
                }
            })
        }
        onPasteRequested: {
            var clipText = Clipboard.text
            if (!clipText || clipText.length === 0) {
                remorsePopup.execute(qsTr("Clipboard is empty"), function() {})
                return
            }
            if (pageView.isAddingNewBlock) {
                pageView.newBlockText = (pageView.newBlockText && pageView.newBlockText.length > 0 ? pageView.newBlockText + "\n" : "") + clipText
            } else if (pageView.editingBlockIndex >= 0) {
                var cur = pageView.editingCurrentText || ""
                var updated = (cur.length > 0 ? cur + "\n" : "") + clipText
                pageView.editingRawText = updated
                pageView.editingCurrentText = updated
            }
        }
    }

    function findConsecutiveListRange(idx) {
        if (!parsedBlocks || idx < 0 || idx >= parsedBlocks.length) return { start: idx, count: 1 }
        var block = parsedBlocks[idx]
        if (!block) return { start: idx, count: 1 }
        var t = block.type
        if (t !== "unordered_list_item" && t !== "ordered_list_item" && t !== "description_list_item") return { start: idx, count: 1 }
        var level = block.level || 0
        var start = idx
        var end = idx
        while (start > 0) {
            var prev = parsedBlocks[start - 1]
            if (prev && prev.type === t && (prev.level || 0) === level) { start--; continue }
            break
        }
        while (end < parsedBlocks.length - 1) {
            var next = parsedBlocks[end + 1]
            if (next && next.type === t && (next.level || 0) === level) { end++; continue }
            break
        }
        return { start: start, count: end - start + 1 }
    }

    function startAddingNewBlock() {
        if (editingBlockIndex >= 0) {
            saveCurrentEditingBlock()
        }
        newBlockText = ""
        isAddingNewBlock = true
        Qt.callLater(function() {
            listView.positionViewAtEnd()
        })
    }

    function saveNewBlock() {
        if (isAddingNewBlock) {
            var trimmed = (newBlockText || "").trim()
            if (trimmed.length > 0) {
                bridge.append_to_current_page(trimmed, false)
            }
            isAddingNewBlock = false
            newBlockText = ""
        }
    }

    function cancelNewBlock() {
        isAddingNewBlock = false
        newBlockText = ""
    }

    function applyNewBlockPrefix(prefix, multiLineList) {
        newBlockText = BlockHtmlUtils.stripAndApplyPrefix(newBlockText || "", prefix)
    }

    function applyBlockPrefix(prefix, multiLineList) {
        var txt = editingCurrentText || ""
        var newText = BlockHtmlUtils.stripAndApplyPrefix(txt, prefix)
        editingRawText = newText
        editingCurrentText = newText
    }

    function saveCurrentEditingBlock() {
        if (editingBlockIndex >= 0) {
            if (editingBlockCount > 1) {
                bridge.save_block_range(editingBlockIndex, editingBlockCount, editingCurrentText)
            } else {
                bridge.save_block(editingBlockIndex, editingCurrentText)
            }
            editingBlockIndex = -1
            editingBlockCount = 1
            editingRawText = ""
            editingCurrentText = ""
        }
    }

    function cancelCurrentEditing() {
        editingBlockIndex = -1
        editingBlockCount = 1
        editingRawText = ""
        editingCurrentText = ""
    }
}
