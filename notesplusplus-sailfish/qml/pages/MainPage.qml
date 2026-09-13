import QtQuick 2.6
import Sailfish.Silica 1.0
import "../components"
import "../js/BlockHtmlUtils.js" as BlockHtmlUtils

Page {
    id: mainPage
    allowedOrientations: Orientation.All

    onStatusChanged: {
        if (status === PageStatus.Active) {
            bridge.load_main_page_data()
        }
    }

    Timer {
        id: mainPageDataPoll
        interval: 50
        running: mainPage.status === PageStatus.Active
        repeat: true
        onTriggered: {
            bridge.poll_main_page_data()
        }
    }

    Timer {
        id: searchPoll
        interval: 100
        repeat: true
        onTriggered: {
            if (bridge.poll_search()) {
                // Initial results delivered, start polling for previews
                searchPoll.stop()
                previewPoll.start()
            }
        }
    }

    Timer {
        id: previewPoll
        interval: 100
        repeat: true
        onTriggered: {
            if (bridge.poll_search_previews()) {
                previewPoll.stop()
            }
        }
    }

    property var parsedSearchResults: {
        var list = []
        for (var i = 0; i < bridge.search_results.length; i++) {
            var str = bridge.search_results[i]
            if (typeof str === "string") {
                try { list.push(JSON.parse(str)) } catch(e) {}
            } else if (typeof str === "object" && str !== null) {
                list.push(str)
            }
        }
        return list
    }

    property var parsedRecentPages: {
        var list = []
        for (var i = 0; i < bridge.recent_pages.length; i++) {
            var str = bridge.recent_pages[i]
            if (typeof str === "string") {
                try { list.push(JSON.parse(str)) } catch(e) {}
            } else if (typeof str === "object" && str !== null) {
                list.push(str)
            }
        }
        return list
    }

    property var parsedGroupedTree: {
        try {
            var raw = bridge.grouped_tree_json
            if (raw && raw.length > 0) {
                return JSON.parse(raw)
            }
        } catch(e) {}
        return []
    }

    property var parsedJournalBlocks: {
        var list = []
        for (var i = 0; i < bridge.journal_blocks.length; i++) {
            var str = bridge.journal_blocks[i]
            if (typeof str === "string") {
                try { list.push(JSON.parse(str)) } catch(e) {}
            } else if (typeof str === "object" && str !== null) {
                list.push(str)
            }
        }
        return list
    }

    property int editingJournalBlockIndex: -1
    property string editingRawText: ""
    property string editingCurrentText: ""
    property bool isAddingJournalBlock: false
    property string newJournalBlockText: ""
    property var currentEditorTextArea: null

    function getActiveEditorTextArea() {
        if (mainPage.isAddingJournalBlock) return journalInlineNewTextArea
        if (mainPage.currentEditorTextArea) return mainPage.currentEditorTextArea
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
            if (mainPage.isAddingJournalBlock) {
                mainPage.newJournalBlockText = target.text
            } else if (mainPage.editingJournalBlockIndex >= 0) {
                mainPage.editingCurrentText = target.text
                mainPage.editingRawText = target.text
            }
            target.forceActiveFocus()
            Qt.callLater(function() {
                if (target) target.forceActiveFocus()
            })
        } else {
            if (mainPage.isAddingJournalBlock) {
                mainPage.newJournalBlockText = (mainPage.newJournalBlockText && mainPage.newJournalBlockText.length > 0 ? mainPage.newJournalBlockText + "\n" : "") + clipText
            } else if (mainPage.editingJournalBlockIndex >= 0) {
                var cur = mainPage.editingCurrentText || ""
                var updated = (cur.length > 0 ? cur + "\n" : "") + clipText
                mainPage.editingRawText = updated
                mainPage.editingCurrentText = updated
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
            if (mainPage.isAddingJournalBlock) {
                mainPage.newJournalBlockText = target.text
            } else if (mainPage.editingJournalBlockIndex >= 0) {
                mainPage.editingCurrentText = target.text
                mainPage.editingRawText = target.text
            }
            target.forceActiveFocus()
            Qt.callLater(function() {
                if (target) target.forceActiveFocus()
            })
        } else {
            if (mainPage.isAddingJournalBlock) {
                mainPage.newJournalBlockText = (mainPage.newJournalBlockText && mainPage.newJournalBlockText.length > 0 ? mainPage.newJournalBlockText + "\n" : "") + formatted
            } else if (mainPage.editingJournalBlockIndex >= 0) {
                var cur = mainPage.editingCurrentText || ""
                var updated = (cur.length > 0 ? cur + "\n" : "") + formatted
                mainPage.editingRawText = updated
                mainPage.editingCurrentText = updated
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
            if (mainPage.isAddingJournalBlock) {
                mainPage.newJournalBlockText = newText
            } else {
                mainPage.editingRawText = newText
                mainPage.editingCurrentText = newText
            }
            target.forceActiveFocus()
            Qt.callLater(function() {
                if (target) target.forceActiveFocus()
            })
        } else {
            if (mainPage.isAddingJournalBlock) {
                mainPage.newJournalBlockText = BlockHtmlUtils.stripAndApplyPrefix(mainPage.newJournalBlockText || "", prefix)
            } else {
                var txt = mainPage.editingCurrentText || ""
                var newText = BlockHtmlUtils.stripAndApplyPrefix(txt, prefix)
                mainPage.editingRawText = newText
                mainPage.editingCurrentText = newText
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
                if (mainPage.isAddingJournalBlock) {
                    mainPage.newJournalBlockText = activeTarget.text
                } else if (mainPage.editingJournalBlockIndex >= 0) {
                    mainPage.editingCurrentText = activeTarget.text
                    mainPage.editingRawText = activeTarget.text
                }
                activeTarget.forceActiveFocus()
                Qt.callLater(function() {
                    if (activeTarget) activeTarget.forceActiveFocus()
                })
            } else {
                if (mainPage.isAddingJournalBlock) {
                    mainPage.newJournalBlockText = (mainPage.newJournalBlockText && mainPage.newJournalBlockText.length > 0 ? mainPage.newJournalBlockText + " " : "") + link
                } else if (mainPage.editingJournalBlockIndex >= 0) {
                    var cur = mainPage.editingCurrentText || ""
                    var updated = (cur.length > 0 ? cur + " " : "") + link
                    mainPage.editingRawText = updated
                    mainPage.editingCurrentText = updated
                }
            }
        })
    }

    function applyJournalBlockPrefix(prefix, multiLineList) {
        applyPrefixToActiveEditor(prefix, multiLineList)
    }

    function saveCurrentEditingJournalBlock() {
        if (editingJournalBlockIndex >= 0) {
            bridge.save_journal_block(editingJournalBlockIndex, editingCurrentText)
            editingJournalBlockIndex = -1
            editingRawText = ""
            editingCurrentText = ""
            currentEditorTextArea = null
        }
    }

    function cancelCurrentJournalEditing() {
        editingJournalBlockIndex = -1
        editingRawText = ""
        editingCurrentText = ""
        currentEditorTextArea = null
    }

    function startAddingJournalBlock() {
        if (editingJournalBlockIndex >= 0) {
            saveCurrentEditingJournalBlock()
        }
        newJournalBlockText = ""
        isAddingJournalBlock = true
        currentEditorTextArea = journalInlineNewTextArea
        Qt.callLater(function() {
            journalInlineNewTextArea.forceActiveFocus()
        })
    }

    function saveNewJournalBlock() {
        if (isAddingJournalBlock) {
            var trimmed = (newJournalBlockText || "").trim()
            if (trimmed.length > 0) {
                bridge.append_to_journal(trimmed, false)
            }
            isAddingJournalBlock = false
            newJournalBlockText = ""
            currentEditorTextArea = null
        }
    }

    function cancelNewJournalBlock() {
        isAddingJournalBlock = false
        newJournalBlockText = ""
        currentEditorTextArea = null
    }

    function applyNewJournalBlockPrefix(prefix, multiLineList) {
        applyPrefixToActiveEditor(prefix, multiLineList)
    }

    function activateSearch() {
        searchField.forceActiveFocus()
    }

    SilicaFlickable {
        anchors.fill: parent
        contentHeight: column.height + Theme.paddingLarge

        PullDownMenu {
            MenuItem {
                text: qsTr("Settings")
                onClicked: {
                    pageStack.push(Qt.resolvedUrl("SettingsPage.qml"))
                }
            }
            MenuItem {
                text: qsTr("AI Assistant")
                visible: (typeof app !== "undefined" && app && app.aiEnabled !== undefined) ? app.aiEnabled : true
                onClicked: {
                    pageStack.push(Qt.resolvedUrl("AssistantPage.qml"))
                }
            }
            MenuItem {
                text: qsTr("New Group")
                onClicked: {
                    var dialog = pageStack.push(Qt.resolvedUrl("NewGroupDialog.qml"))
                    dialog.accepted.connect(function() {
                        if (dialog.groupName.length > 0) {
                            bridge.create_group("", dialog.groupName)
                        }
                    })
                }
            }
            MenuItem {
                text: qsTr("New Note")
                onClicked: {
                    var dialog = pageStack.push(Qt.resolvedUrl("NewPageDialog.qml"))
                    dialog.accepted.connect(function() {
                        if (dialog.pageName.length > 0) {
                            bridge.create_page(dialog.pageName)
                        }
                    })
                }
            }
            MenuItem {
                text: qsTr("Journal")
                visible: (typeof app !== "undefined" && app && app.journalEnabled !== undefined) ? app.journalEnabled : true
                onClicked: {
                    pageStack.push(Qt.resolvedUrl("PageView.qml"), {
                        pageName: "Journal"
                    })
                    bridge.load_page("Journal")
                }
            }
        }

        Column {
            id: column
            width: parent.width

            PageHeader {
                title: "Notes++"
            }

            Item {
                width: parent.width
                height: bridge.web_server_running ? (webStatusRow.height + Theme.paddingSmall) : 0
                visible: bridge.web_server_running
                clip: true

                Behavior on height { NumberAnimation { duration: 150 } }

                BackgroundItem {
                    id: webStatusRow
                    anchors.centerIn: parent
                    width: parent.width - Theme.horizontalPageMargin * 2
                    height: Theme.itemSizeExtraSmall

                    Row {
                        anchors.centerIn: parent
                        spacing: Theme.paddingSmall

                        Rectangle {
                            width: Theme.paddingSmall
                            height: Theme.paddingSmall
                            radius: width / 2
                            color: "#4cd964"
                            anchors.verticalCenter: parent.verticalCenter
                        }

                        Label {
                            text: qsTr("Web Service: ") + bridge.web_server_url
                            font.pixelSize: Theme.fontSizeExtraSmall
                            color: Theme.secondaryHighlightColor
                            anchors.verticalCenter: parent.verticalCenter
                        }
                    }

                    onClicked: {
                        var url = bridge.web_server_url
                        if (url) {
                            Clipboard.text = url
                        }
                    }
                }
            }

            SearchField {
                id: searchField
                width: parent.width
                placeholderText: qsTr("Search your notes...")
                onTextChanged: {
                    if (text.length > 0) {
                        bridge.do_search(text)
                        searchPoll.start()
                    } else {
                        searchPoll.stop()
                        bridge.search("")
                    }
                }
            }

            // Search results section
            SectionHeader {
                text: qsTr("Search Results (%1)").arg(parsedSearchResults.length)
                visible: searchField.text.length > 0
            }

            NoteCardGrid {
                id: searchGrid
                width: parent.width
                isPortraitOrientation: isPortrait
                model: searchField.text.length > 0 ? parsedSearchResults : []
                searchTerm: searchField.text
                visible: searchField.text.length > 0 && parsedSearchResults.length > 0
                onItemClicked: function(itemData, itemIndex) {
                    var target = itemData.full_path || itemData.name
                    pageStack.push(Qt.resolvedUrl("PageView.qml"), {
                        pageName: itemData.name,
                        searchTerm: searchField.text
                    })
                    bridge.load_page(target)
                }
            }

            // No search results empty state
            Label {
                x: Theme.horizontalPageMargin
                width: parent.width - Theme.horizontalPageMargin * 2
                visible: searchField.text.length > 0 && parsedSearchResults.length === 0
                text: qsTr("No results found for \"%1\"").arg(searchField.text)
                color: Theme.secondaryColor
                font.pixelSize: Theme.fontSizeSmall
                font.italic: true
                wrapMode: Text.Wrap
            }

            // Journal section
            SectionHeader {
                text: qsTr("Journal")
                visible: (typeof app !== "undefined" && app && app.journalEnabled !== undefined ? app.journalEnabled : true) && searchField.text.length === 0
            }

            Column {
                width: parent.width
                spacing: Theme.paddingSmall
                visible: (typeof app !== "undefined" && app && app.journalEnabled !== undefined ? app.journalEnabled : true) && searchField.text.length === 0

                Repeater {
                    model: searchField.text.length === 0 ? parsedJournalBlocks.length : 0

                    delegate: BlockDelegate {
                        width: parent.width
                        blockData: parsedJournalBlocks[index]
                        blockIndex: index
                        interactive: true
                        searchTerm: ""
                        isEditing: mainPage.editingJournalBlockIndex === index
                        editingRawText: (mainPage.editingJournalBlockIndex === index) ? mainPage.editingRawText : ""

                        onEditRequested: function(idx, raw) {
                            if (mainPage.isAddingJournalBlock) {
                                mainPage.saveNewJournalBlock()
                            }
                            var blockObj = parsedJournalBlocks[idx]
                            var fallback = raw || (blockObj ? (blockObj.raw || blockObj.text || "") : "")
                            mainPage.editingRawText = fallback
                            mainPage.editingCurrentText = fallback
                            mainPage.editingJournalBlockIndex = idx
                        }

                        onEditorReady: function(ta) {
                            mainPage.currentEditorTextArea = ta
                        }

                        onTextModified: function(idx, newText) {
                            if (mainPage.editingJournalBlockIndex === idx) {
                                mainPage.editingCurrentText = newText
                            }
                        }

                        onSaveRequested: function(idx, newRaw) {
                            bridge.save_journal_block(idx, newRaw)
                            mainPage.editingJournalBlockIndex = -1
                            mainPage.editingRawText = ""
                            mainPage.editingCurrentText = ""
                            mainPage.currentEditorTextArea = null
                        }

                        onCancelEditRequested: function() {
                            mainPage.cancelCurrentJournalEditing()
                        }

                        onCheckboxToggled: function(idx, itemPath) {
                            bridge.toggle_journal_checkbox(idx, itemPath)
                        }

                        onXrefActivated: function(target) {
                            pageStack.push(Qt.resolvedUrl("PageView.qml"), {
                                pageName: target
                            })
                            bridge.load_page(target)
                        }
                    }
                }

                // Inline editor for adding to today's journal
                Item {
                    id: journalNewBlockEditor
                    width: parent.width
                    height: mainPage.isAddingJournalBlock ? (journalInlineNewTextArea.implicitHeight + Theme.paddingMedium) : 0
                    visible: mainPage.isAddingJournalBlock

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
                        id: journalInlineNewTextArea
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.leftMargin: Theme.horizontalPageMargin
                        anchors.rightMargin: Theme.horizontalPageMargin + Theme.itemSizeMedium
                        anchors.verticalCenter: parent.verticalCenter
                        text: mainPage.newJournalBlockText
                        placeholderText: qsTr("Add journal entry, task (* [ ]), or note...")
                        font.pixelSize: app.scaledFontSize(Theme.fontSizeMedium)
                        color: Theme.primaryColor
                        background: null
                        onTextChanged: {
                            if (mainPage.isAddingJournalBlock) {
                                mainPage.newJournalBlockText = text
                            }
                        }
                    }
                }

                // Free area tap target to add today's journal entry
                BackgroundItem {
                    width: parent.width
                    height: Theme.itemSizeSmall
                    visible: searchField.text.length === 0 && mainPage.editingJournalBlockIndex < 0 && !mainPage.isAddingJournalBlock
                    onClicked: {
                        mainPage.startAddingJournalBlock()
                    }

                    Label {
                        anchors.left: parent.left
                        anchors.leftMargin: Theme.horizontalPageMargin
                        anchors.verticalCenter: parent.verticalCenter
                        text: qsTr("+ Add to today's journal...")
                        font.pixelSize: Theme.fontSizeSmall
                        color: Theme.rgba(Theme.primaryColor, 0.5)
                    }
                }

                // Open Full Journal Link / Button
                BackgroundItem {
                    width: parent.width - Theme.horizontalPageMargin * 2
                    anchors.horizontalCenter: parent.horizontalCenter
                    height: Theme.itemSizeExtraSmall
                    visible: searchField.text.length === 0 && mainPage.editingJournalBlockIndex < 0 && !mainPage.isAddingJournalBlock
                    onClicked: {
                        pageStack.push(Qt.resolvedUrl("PageView.qml"), {
                            pageName: "Journal"
                        })
                        bridge.load_page("Journal")
                    }

                    Label {
                        anchors.right: parent.right
                        anchors.verticalCenter: parent.verticalCenter
                        text: qsTr("Full Journal →")
                        font.pixelSize: Theme.fontSizeExtraSmall
                        color: Theme.highlightColor
                    }
                }
            }

            // Grouped Notes Section
            Repeater {
                model: searchField.text.length === 0 ? parsedGroupedTree : []
                delegate: NoteGroupSection {
                    width: parent.width
                    groupData: modelData
                    isPortraitOrientation: isPortrait
                    remorsePopupRef: remorsePopup
                }
            }

            // Empty wiki guidance
            Column {
                width: parent.width - Theme.horizontalPageMargin * 2
                anchors.horizontalCenter: parent.horizontalCenter
                spacing: Theme.paddingMedium
                visible: searchField.text.length === 0 && parsedRecentPages.length === 0 && parsedGroupedTree.length === 0

                Label {
                    width: parent.width
                    text: qsTr("Welcome to Notes++")
                    color: Theme.highlightColor
                    font.pixelSize: Theme.fontSizeLarge
                    font.bold: true
                    wrapMode: Text.Wrap
                }

                Label {
                    width: parent.width
                    text: qsTr("Pull down to create your first page, or start a journal entry below. Pages use AsciiDoc markup for rich formatting, code blocks, and cross-links.")
                    color: Theme.secondaryColor
                    font.pixelSize: Theme.fontSizeSmall
                    wrapMode: Text.Wrap
                }
            }
        }
    }

    RemorsePopup {
        id: remorsePopup
    }

    // Stationary floating action sidebar for in-place journal editing (transparent, non-moving)
    InPlaceEditSidebar {
        id: inPlaceSidebar
        anchors.right: parent.right
        anchors.rightMargin: Theme.paddingMedium
        anchors.verticalCenter: parent.verticalCenter
        visible: (typeof app !== "undefined" && app && app.journalEnabled !== undefined ? app.journalEnabled : true) && (mainPage.editingJournalBlockIndex >= 0 || mainPage.isAddingJournalBlock)
        onAccepted: {
            if (mainPage.isAddingJournalBlock) {
                mainPage.saveNewJournalBlock()
            } else {
                mainPage.saveCurrentEditingJournalBlock()
            }
        }
        onCanceled: {
            if (mainPage.isAddingJournalBlock) {
                mainPage.cancelNewJournalBlock()
            } else {
                mainPage.cancelCurrentJournalEditing()
            }
        }
        onPrefixRequested: function(prefix, multiLine) {
            mainPage.applyPrefixToActiveEditor(prefix, multiLine)
        }
        onLinkRequested: {
            mainPage.insertLinkIntoActiveEditor()
        }
        onPasteRequested: {
            mainPage.pasteTextIntoActiveEditor(Clipboard.text)
        }
        onPasteSpecialRequested: function(prefix, multiLine) {
            mainPage.pasteSpecialIntoActiveEditor(prefix, multiLine)
        }
        onRefocusRequested: {
            mainPage.refocusActiveEditor()
        }
    }
}
