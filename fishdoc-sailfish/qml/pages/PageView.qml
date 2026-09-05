import QtQuick 2.6
import Sailfish.Silica 1.0
import "../components"

Page {
    id: pageView
    allowedOrientations: Orientation.All

    property string pageName: bridge.current_page_name
    property string searchTerm: ""
    property int targetBlockIndex: -1
    property bool hasScrolledToSearchTerm: false

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
        model: bridge.blocks_version >= 0 ? bridge.current_blocks : []

        PullDownMenu {
            MenuItem {
                text: "Edit Source"
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
                text: "Settings"
                onClicked: {
                    pageStack.push(Qt.resolvedUrl("SettingsPage.qml"))
                }
            }
            MenuItem {
                text: "Open in Browser"
                onClicked: {
                    bridge.open_in_browser(pageName)
                }
            }
            MenuItem {
                text: "Export to HTML5"
                onClicked: {
                    var path = bridge.export_html(pageName)
                    if (path) {
                        remorsePopup.execute("Exported to " + path, function() {})
                    }
                }
            }
            MenuItem {
                text: "Delete Page"
                visible: !bridge.is_journal_page && pageName !== "Journal" && pageName !== "journal"
                onClicked: {
                    remorsePopup.execute("Deleting page", function() {
                        bridge.delete_page(pageName)
                        pageStack.pop()
                    })
                }
            }
        }

        header: PageHeader {
            title: pageName
        }

        footer: Item {
            width: listView.width
            height: Theme.paddingLarge * 2
        }

        delegate: BlockDelegate {
            width: listView.width
            blockData: (pageView.parsedBlocks && pageView.parsedBlocks[index]) ? pageView.parsedBlocks[index] : (modelData ? JSON.parse(modelData) : ({}))
            allBlocks: pageView.parsedBlocks
            blockIndex: index
            searchTerm: pageView.searchTerm
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
                var blocks = pageView.parsedBlocks || []
                if (idx < 0 || idx >= blocks.length) return

                var startIdx = idx
                var endIdx = idx

                if (blocks[idx].type === "heading") {
                    startIdx = idx
                    endIdx = idx
                    while (endIdx + 1 < blocks.length && blocks[endIdx + 1].type !== "heading") {
                        endIdx++
                    }
                } else {
                    startIdx = idx
                    while (startIdx > 0 && blocks[startIdx - 1].type !== "heading") {
                        startIdx--
                    }
                    endIdx = idx
                    while (endIdx + 1 < blocks.length && blocks[endIdx + 1].type !== "heading") {
                        endIdx++
                    }
                }

                var combinedRaw = ""
                var cursorOffset = 0
                for (var k = startIdx; k <= endIdx; k++) {
                    if (k === idx) {
                        cursorOffset = combinedRaw.length
                    }
                    var r = blocks[k].raw !== undefined ? blocks[k].raw : (blocks[k].raw_text || "")
                    if (combinedRaw.length > 0) {
                        combinedRaw += "\n" + r
                    } else {
                        combinedRaw = r
                    }
                }

                var count = endIdx - startIdx + 1
                var editor = pageStack.push(Qt.resolvedUrl("../components/BlockEditor.qml"), {
                    blockIndex: startIdx,
                    blockCount: count,
                    rawText: combinedRaw,
                    initialCursorPosition: cursorOffset
                })
                editor.accepted.connect(function() {
                    bridge.load_page(pageName)
                })
            }
        }

        RemorsePopup { id: remorsePopup }
    }
}
