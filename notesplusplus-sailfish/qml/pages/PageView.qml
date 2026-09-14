import QtQuick 2.6
import Sailfish.Silica 1.0
import "../components"
import "../js/BlockHtmlUtils.js" as BlockHtmlUtils

Page {
    id: pageView
    allowedOrientations: Orientation.All

    property string pageName: bridge.current_page_name
    property string pageFullPath: bridge.current_page_full_path.length > 0
                                  ? bridge.current_page_full_path
                                  : (bridge.current_page_group_path.length > 0
                                     ? bridge.current_page_group_path + "/" + pageName
                                     : pageName)
    property string pageFilePath: bridge.current_page_file_path || pageName
    property string pageGroupPath: bridge.current_page_group_path
    property bool isJournalPage: bridge.is_journal_page
    property string initialTargetPage: ""
    property string initialAnchor: ""
    property string recordedPagePath: ""
    property string findInPageTerm: ""
    property bool showFindBar: false
    property var allFindMatches: []
    property int currentFindMatchIndex: -1
    readonly property var currentFindMatch: (currentFindMatchIndex >= 0 && currentFindMatchIndex < allFindMatches.length)
                                            ? allFindMatches[currentFindMatchIndex]
                                            : null
    property int editingBlockIndex: -1
    property int editingBlockCount: 1
    property string editingRawText: ""
    property string editingCurrentText: ""
    property bool isAddingNewBlock: false
    property string newBlockText: ""
    property var currentEditorTextArea: null

    property var collapsedTocBlocks: ({})

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

    onParsedBlocksChanged: {
        if (findInPageTerm.length > 0) {
            updateFindMatches()
        }
        if (initialAnchor && initialAnchor.length > 0 && parsedBlocks && parsedBlocks.length > 0) {
            var anchor = initialAnchor
            initialAnchor = ""
            Qt.callLater(function() {
                pageView.jumpToAnchor(anchor)
            })
        }
    }

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

    function jumpToAnchor(anchor) {
        if (!anchor || anchor.length === 0 || !parsedBlocks || parsedBlocks.length === 0) return
        var cleanAnchor = anchor.replace(/^#+/, '').trim().toLowerCase()
        if (cleanAnchor.length === 0) return

        for (var i = 0; i < parsedBlocks.length; i++) {
            var b = parsedBlocks[i]
            if (!b) continue
            if (b.id && String(b.id).toLowerCase() === cleanAnchor) {
                listView.positionViewAtIndex(i, ListView.Beginning)
                return
            }
            var title = (b.title || "").trim().toLowerCase()
            if (title.length > 0) {
                if (title === cleanAnchor ||
                    title.replace(/\s+/g, '-') === cleanAnchor ||
                    title.replace(/[-_]/g, ' ') === cleanAnchor.replace(/[-_]/g, ' ')) {
                    listView.positionViewAtIndex(i, ListView.Beginning)
                    return
                }
            }
            var raw = b.raw || b.raw_text || ""
            if (raw.indexOf("[[" + cleanAnchor + "]]") >= 0 ||
                raw.indexOf("[#" + cleanAnchor + "]") >= 0 ||
                raw.toLowerCase().indexOf("[[" + cleanAnchor + "]]") >= 0 ||
                raw.toLowerCase().indexOf("[#" + cleanAnchor + "]") >= 0) {
                listView.positionViewAtIndex(i, ListView.Beginning)
                return
            }
        }
    }

    function handleXref(target) {
        if (!target || target.length === 0) return
        if (pageView.editingBlockIndex >= 0) {
            pageView.saveCurrentEditingBlock()
        }
        if (pageView.isAddingNewBlock) {
            pageView.saveNewBlock()
        }

        var hashIdx = target.indexOf("#")
        var pagePart = (hashIdx >= 0) ? target.substring(0, hashIdx) : target
        var anchor = (hashIdx >= 0) ? target.substring(hashIdx + 1) : ""

        if (pagePart.indexOf(".adoc") === pagePart.length - 5 && pagePart.length >= 5) {
            pagePart = pagePart.substring(0, pagePart.length - 5)
        }

        var myPath = pageView.recordedPagePath || pageView.pageFullPath || pageView.pageName

        if (pagePart.length === 0 || pagePart === myPath || pagePart === pageView.pageName || pagePart === pageView.pageFullPath) {
            if (anchor.length > 0) {
                jumpToAnchor(anchor)
            }
            return
        }

        pageStack.push(Qt.resolvedUrl("PageView.qml"), {
            initialTargetPage: pagePart,
            initialAnchor: anchor
        })
        bridge.load_page(pagePart)
    }

    Connections {
        target: bridge
        onCurrent_page_full_path_changed: {
            if ((pageView.status === PageStatus.Active || pageView.status === PageStatus.Activating) && bridge.current_page_full_path.length > 0) {
                pageView.recordedPagePath = bridge.current_page_full_path
            }
        }
    }

    onStatusChanged: {
        if (status === PageStatus.Activating || status === PageStatus.Active) {
            var target = (recordedPagePath && recordedPagePath.length > 0) ? recordedPagePath : initialTargetPage
            if (target && target.length > 0 && bridge.current_page_full_path !== target && bridge.current_page_name !== target) {
                bridge.load_page(target)
            }
        }
    }

    Component.onCompleted: {
        if (initialTargetPage.length > 0) {
            recordedPagePath = initialTargetPage
            if (bridge.current_page_full_path !== initialTargetPage && bridge.current_page_name !== initialTargetPage) {
                bridge.load_page(initialTargetPage)
            }
        } else if (bridge.current_page_full_path.length > 0) {
            recordedPagePath = bridge.current_page_full_path
        }
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

    FindInPageBar {
        id: findInPageBar
        anchors.bottom: parent.bottom
        anchors.bottomMargin: Theme.paddingMedium
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.leftMargin: Theme.horizontalPageMargin
        anchors.rightMargin: Theme.horizontalPageMargin
        z: 10
        visible: pageView.showFindBar
        searchTerm: pageView.findInPageTerm
        currentMatchIndex: pageView.currentFindMatchIndex
        totalMatches: pageView.allFindMatches.length

        onTextChanged: function(text) {
            if (pageView.findInPageTerm !== text) {
                pageView.findInPageTerm = text
            }
        }
        onNextClicked: {
            pageView.findNext()
        }
        onPreviousClicked: {
            pageView.findPrevious()
        }
        onCloseClicked: {
            pageView.closeFindBar()
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
                    if (pageView.showFindBar) {
                        pageView.closeFindBar()
                    } else {
                        pageView.openFindBar()
                    }
                }
            }
            MenuItem {
                text: qsTr("Ask AI Assistant")
                visible: (typeof app !== "undefined" && app && app.aiEnabled !== undefined) ? app.aiEnabled : true
                onClicked: {
                    var targetPath = pageView.pageFullPath
                    var content = bridge.get_page_source(targetPath)
                    var fname = targetPath.indexOf(".adoc") >= 0 ? targetPath : (targetPath + ".adoc")
                    app.openAssistant(fname, content)
                }
            }
            MenuItem {
                text: qsTr("Edit Source")
                onClicked: {
                    var targetPath = pageView.pageFullPath
                    var editor = pageStack.push(Qt.resolvedUrl("PageSourceEditor.qml"), {
                        pageName: targetPath
                    })
                    editor.accepted.connect(function() {
                        bridge.load_page(targetPath)
                    })
                }
            }
            MenuItem {
                text: qsTr("Copy Page URL")
                onClicked: {
                    var url = bridge.open_in_browser(pageView.pageFullPath)
                    if (url) {
                        Clipboard.text = url
                        remorsePopup.execute(qsTr("Copied: ") + url, function() {}, 3000)
                    }
                }
            }
            MenuItem {
                text: qsTr("Move to Group...")
                visible: !pageView.isJournalPage && pageName !== "Journal" && pageName !== "journal"
                onClicked: {
                    var fullPath = pageView.pageFullPath
                    var dialog = pageStack.push(Qt.resolvedUrl("MovePageDialog.qml"), {
                        pageFullPath: fullPath,
                        pageTitle: pageName,
                        currentGroup: pageView.pageGroupPath
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
                visible: !pageView.isJournalPage && pageName !== "Journal" && pageName !== "journal"
                onClicked: {
                    var fullPath = pageView.pageFullPath
                    remorsePopup.execute(qsTr("Deleting page"), function() {
                        bridge.delete_page(fullPath)
                        pageStack.pop()
                    })
                }
            }
        }

        header: Column {
            width: listView.width

            PageHeader {
                title: pageView.pageFilePath
            }

            Label {
                id: breadcrumbLabel
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.leftMargin: Theme.horizontalPageMargin
                anchors.rightMargin: Theme.horizontalPageMargin
                visible: pageView.pageGroupPath.length > 0 && !pageView.isJournalPage
                text: pageView.pageGroupPath.split("/").join(" › ")
                color: Theme.highlightColor
                font.pixelSize: Theme.fontSizeExtraSmall
                truncationMode: TruncationMode.Fade
            }
        }

        footer: Item {
            id: listFooter
            width: listView.width
            height: pageView.isAddingNewBlock
                ? (newBlockEditor.height + Theme.paddingLarge * 2)
                : Math.max(Theme.itemSizeExtraLarge * 2, listView.height - (Theme.itemSizeLarge + Theme.paddingLarge))

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
            searchTerm: pageView.findInPageTerm
            isCurrentMatchBlock: Boolean(pageView.currentFindMatch && pageView.currentFindMatch.blockIndex === index)
            activeMatchIndexInBlock: (pageView.currentFindMatch && pageView.currentFindMatch.blockIndex === index)
                                     ? pageView.currentFindMatch.matchInBlock
                                     : -1
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
                pageView.handleXref(target)
            }
            onCheckboxToggled: function(idx, itemPath) {
                pageView.toggleParsedBlockCheckbox(idx, itemPath)
                bridge.toggle_checkbox(idx, itemPath)
            }
            onEditRequested: function(idx, raw) {
                if (pageView.isAddingNewBlock) {
                    pageView.saveNewBlock()
                }
                var range = pageView.findEditRange(idx)
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
            onEditorReady: function(ta) {
                pageView.currentEditorTextArea = ta
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
                pageView.currentEditorTextArea = null
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
        visible: (pageView.editingBlockIndex >= 0 || pageView.isAddingNewBlock) && !pageView.showFindBar
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
            pageView.applyPrefixToActiveEditor(prefix, multiLine)
        }
        onLinkRequested: {
            pageView.insertLinkIntoActiveEditor()
        }
        onPasteRequested: {
            pageView.pasteTextIntoActiveEditor(Clipboard.text)
        }
        onPasteSpecialRequested: function(prefix, multiLine) {
            pageView.pasteSpecialIntoActiveEditor(prefix, multiLine)
        }
        onRefocusRequested: {
            pageView.refocusActiveEditor()
        }
    }

    function findEditRange(idx) {
        if (!parsedBlocks || idx < 0 || idx >= parsedBlocks.length) return { start: idx, count: 1 }
        var block = parsedBlocks[idx]
        if (!block) return { start: idx, count: 1 }
        var t = block.type

        // Heading: include everything until next heading of equal-or-higher level
        if (t === "heading") {
            var level = block.level || 1
            var end = idx
            while (end < parsedBlocks.length - 1) {
                var next = parsedBlocks[end + 1]
                if (next && next.type === "heading" && (next.level || 1) <= level) break
                end++
            }
            return { start: idx, count: end - idx + 1 }
        }

        // List item: include all contiguous list items (any type/level) + EmptyLines between them
        var listTypes = ["unordered_list_item", "ordered_list_item",
                         "description_list_item", "callout_list_item"]
        if (listTypes.indexOf(t) >= 0) {
            var start = idx
            while (start > 0) {
                var prev = parsedBlocks[start - 1]
                if (!prev) break
                if (listTypes.indexOf(prev.type) >= 0 || prev.type === "empty_line") {
                    start--
                    continue
                }
                break
            }
            var end = idx
            while (end < parsedBlocks.length - 1) {
                var next = parsedBlocks[end + 1]
                if (!next) break
                if (listTypes.indexOf(next.type) >= 0 || next.type === "empty_line") {
                    end++
                    continue
                }
                break
            }
            return { start: start, count: end - start + 1 }
        }

        // All other blocks: single-block edit
        return { start: idx, count: 1 }
    }

    function getActiveEditorTextArea() {
        if (pageView.isAddingNewBlock) return inlineNewTextArea
        if (pageView.currentEditorTextArea) return pageView.currentEditorTextArea
        return null
    }

    function refocusActiveEditor() {
        var target = getActiveEditorTextArea()
        if (target) {
            target.forceActiveFocus()
            Qt.callLater(function() {
                if (target) target.forceActiveFocus()
            })
        }
    }

    function pasteTextIntoActiveEditor(clipText) {
        if (!clipText || clipText.length === 0) {
            remorsePopup.execute(qsTr("Clipboard is empty"), function() {})
            return
        }
        var target = getActiveEditorTextArea()
        if (target) {
            var start = Math.min(target.selectionStart, target.selectionEnd)
            var end = Math.max(target.selectionStart, target.selectionEnd)
            var txt = target.text || ""
            var pos = target.cursorPosition
            if (start !== end && start >= 0 && end <= txt.length) {
                var before = txt.substring(0, start)
                var after = txt.substring(end)
                target.text = before + clipText + after
                target.cursorPosition = start + clipText.length
            } else {
                if (pos < 0 || pos > txt.length) pos = txt.length
                var before = txt.substring(0, pos)
                var after = txt.substring(pos)
                target.text = before + clipText + after
                target.cursorPosition = pos + clipText.length
            }
            if (pageView.isAddingNewBlock) {
                pageView.newBlockText = target.text
            } else if (pageView.editingBlockIndex >= 0) {
                pageView.editingCurrentText = target.text
                pageView.editingRawText = target.text
            }
            target.forceActiveFocus()
            Qt.callLater(function() {
                if (target) target.forceActiveFocus()
            })
        } else {
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

    function pasteSpecialIntoActiveEditor(prefix, multiLine) {
        var clipText = Clipboard.text
        if (!clipText || clipText.length === 0) {
            remorsePopup.execute(qsTr("Clipboard is empty"), function() {})
            return
        }
        var formatted = BlockHtmlUtils.formatPasteWithPrefix(clipText, prefix)
        var target = getActiveEditorTextArea()
        if (target) {
            var start = Math.min(target.selectionStart, target.selectionEnd)
            var end = Math.max(target.selectionStart, target.selectionEnd)
            var txt = target.text || ""
            var pos = target.cursorPosition
            var inserted = formatted
            if (start !== end && start >= 0 && end <= txt.length) {
                var before = txt.substring(0, start)
                var after = txt.substring(end)
                target.text = before + inserted + after
                target.cursorPosition = start + inserted.length
            } else {
                if (pos < 0 || pos > txt.length) pos = txt.length
                var prefixNewline = ""
                if (pos > 0 && txt.charAt(pos - 1) !== '\n') {
                    prefixNewline = "\n"
                }
                var toInsert = prefixNewline + inserted
                var before = txt.substring(0, pos)
                var after = txt.substring(pos)
                target.text = before + toInsert + after
                target.cursorPosition = pos + toInsert.length
            }
            if (pageView.isAddingNewBlock) {
                pageView.newBlockText = target.text
            } else if (pageView.editingBlockIndex >= 0) {
                pageView.editingCurrentText = target.text
                pageView.editingRawText = target.text
            }
            target.forceActiveFocus()
            Qt.callLater(function() {
                if (target) target.forceActiveFocus()
            })
        } else {
            if (pageView.isAddingNewBlock) {
                pageView.newBlockText = (pageView.newBlockText && pageView.newBlockText.length > 0 ? pageView.newBlockText + "\n" : "") + formatted
            } else if (pageView.editingBlockIndex >= 0) {
                var cur = pageView.editingCurrentText || ""
                var updated = (cur.length > 0 ? cur + "\n" : "") + formatted
                pageView.editingRawText = updated
                pageView.editingCurrentText = updated
            }
        }
    }

    function applyPrefixToActiveEditor(prefix, multiLineList) {
        var target = getActiveEditorTextArea()
        if (target) {
            var curPos = target.cursorPosition
            var txt = target.text || ""
            var newText = BlockHtmlUtils.stripAndApplyPrefix(txt, prefix)
            target.text = newText
            var diff = newText.length - txt.length
            var newPos = Math.max(0, Math.min(newText.length, curPos + diff))
            target.cursorPosition = newPos
            if (pageView.isAddingNewBlock) {
                pageView.newBlockText = newText
            } else {
                pageView.editingRawText = newText
                pageView.editingCurrentText = newText
            }
            target.forceActiveFocus()
            Qt.callLater(function() {
                if (target) target.forceActiveFocus()
            })
        } else {
            if (pageView.isAddingNewBlock) {
                pageView.newBlockText = BlockHtmlUtils.stripAndApplyPrefix(pageView.newBlockText || "", prefix)
            } else {
                var txt = pageView.editingCurrentText || ""
                var newText = BlockHtmlUtils.stripAndApplyPrefix(txt, prefix)
                pageView.editingRawText = newText
                pageView.editingCurrentText = newText
            }
        }
    }

    function insertLinkIntoActiveEditor() {
        var target = getActiveEditorTextArea()
        var sel = ""
        if (target) {
            var start = Math.min(target.selectionStart, target.selectionEnd)
            var end = Math.max(target.selectionStart, target.selectionEnd)
            var txt = target.text || ""
            if (start !== end && start >= 0 && end <= txt.length) {
                sel = txt.substring(start, end)
            }
        }
        var dialog = pageStack.push(Qt.resolvedUrl("PageLinkDialog.qml"), {
            selectedText: sel
        })
        dialog.accepted.connect(function() {
            var link = dialog.formattedLink
            if (!link) return
            var activeTarget = getActiveEditorTextArea()
            if (activeTarget) {
                var start = Math.min(activeTarget.selectionStart, activeTarget.selectionEnd)
                var end = Math.max(activeTarget.selectionStart, activeTarget.selectionEnd)
                var txt = activeTarget.text || ""
                var pos = activeTarget.cursorPosition
                if (start !== end && start >= 0 && end <= txt.length) {
                    var before = txt.substring(0, start)
                    var after = txt.substring(end)
                    activeTarget.text = before + link + after
                    activeTarget.cursorPosition = start + link.length
                } else {
                    if (pos < 0 || pos > txt.length) pos = txt.length
                    var before = txt.substring(0, pos)
                    var after = txt.substring(pos)
                    activeTarget.text = before + link + after
                    activeTarget.cursorPosition = pos + link.length
                }
                if (pageView.isAddingNewBlock) {
                    pageView.newBlockText = activeTarget.text
                } else if (pageView.editingBlockIndex >= 0) {
                    pageView.editingCurrentText = activeTarget.text
                    pageView.editingRawText = activeTarget.text
                }
                activeTarget.forceActiveFocus()
                Qt.callLater(function() {
                    if (activeTarget) activeTarget.forceActiveFocus()
                })
            } else {
                if (pageView.isAddingNewBlock) {
                    pageView.newBlockText = (pageView.newBlockText && pageView.newBlockText.length > 0 ? pageView.newBlockText + " " : "") + link
                } else if (pageView.editingBlockIndex >= 0) {
                    var cur = pageView.editingCurrentText || ""
                    var updated = (cur.length > 0 ? cur + " " : "") + link
                    pageView.editingRawText = updated
                    pageView.editingCurrentText = updated
                }
            }
        })
    }

    function startAddingNewBlock() {
        if (editingBlockIndex >= 0) {
            saveCurrentEditingBlock()
        }
        newBlockText = ""
        isAddingNewBlock = true
        currentEditorTextArea = inlineNewTextArea
        Qt.callLater(function() {
            listView.positionViewAtEnd()
            inlineNewTextArea.forceActiveFocus()
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
            currentEditorTextArea = null
        }
    }

    function cancelNewBlock() {
        isAddingNewBlock = false
        newBlockText = ""
        currentEditorTextArea = null
    }

    function applyNewBlockPrefix(prefix, multiLineList) {
        applyPrefixToActiveEditor(prefix, multiLineList)
    }

    function applyBlockPrefix(prefix, multiLineList) {
        applyPrefixToActiveEditor(prefix, multiLineList)
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
            currentEditorTextArea = null
        }
    }

    function cancelCurrentEditing() {
        editingBlockIndex = -1
        editingBlockCount = 1
        editingRawText = ""
        editingCurrentText = ""
        currentEditorTextArea = null
    }
}
