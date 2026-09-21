import QtQuick 2.6
import Sailfish.Silica 1.0
import "../components/common"
import "../components/editor"
import "../js/BlockHtmlUtils.js" as BlockHtmlUtils
import "../js/EditorHelpers.js" as EH

Page {
    id: mainPage
    allowedOrientations: Orientation.All

    onStatusChanged: {
        if (status === PageStatus.Active) {
            bridge.load_main_page_data()
        } else if (status === PageStatus.Deactivating || status === PageStatus.Inactive) {
            voiceController.cancelRecording()
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
    property bool showJournalDiscardConfirmation: false

    function getActiveEditorTextArea() {
        if (mainPage.isAddingJournalBlock) return journalInlineNewTextArea
        if (mainPage.currentEditorTextArea) return mainPage.currentEditorTextArea
        return null
    }

    function refocusActiveEditor() {
        var target = getActiveEditorTextArea()
        if (target) {
            target.forceActiveFocus()
        }
    }

    function openElementPicker() {
        var dialog = pageStack.push(Qt.resolvedUrl("../dialogs/AsciiDocElementPickerDialog.qml"))
        dialog.insertSnippet.connect(function(snippet) {
            insertSnippetIntoActiveEditor(snippet)
        })
        dialog.statusChanged.connect(function() {
            if (dialog.status === PageStatus.Inactive)
                mainPage.refocusActiveEditor()
        })
    }

    function _editorCtx() {
        return {
            target: getActiveEditorTextArea(),
            isAdding: mainPage.isAddingJournalBlock,
            newBlockText: mainPage.newJournalBlockText,
            editingIndex: mainPage.editingJournalBlockIndex,
            editingText: mainPage.editingJournalCurrentText || mainPage.editingCurrentText,
            editingRaw: mainPage.editingJournalRawText || mainPage.editingRawText,
            focusCb: function(t) {
                if (t) {
                    t.forceActiveFocus()
                }
            }
        }
    }
    function _syncFromCtx(ctx) {
        if (ctx.isAdding) {
            mainPage.newJournalBlockText = ctx.newBlockText
        } else if (ctx.editingIndex >= 0) {
            mainPage.editingJournalCurrentText = ctx.editingText
            mainPage.editingJournalRawText = ctx.editingRaw
            mainPage.editingCurrentText = ctx.editingText
            mainPage.editingRawText = ctx.editingRaw
        }
    }

    function insertSnippetIntoActiveEditor(snippet) {
        var ctx = _editorCtx()
        EH.insertSnippet(ctx, snippet)
        _syncFromCtx(ctx)
    }

    function pasteTextIntoActiveEditor(clipText) {
        if (!clipText || clipText.length === 0) {
            remorsePopup.execute(qsTr("Clipboard is empty"), function() {})
            return
        }
        var ctx = _editorCtx()
        EH.pasteText(ctx, clipText)
        _syncFromCtx(ctx)
    }

    function pasteSpecialIntoActiveEditor(prefix, multiLine) {
        var clipText = Clipboard.text
        if (!clipText || clipText.length === 0) {
            remorsePopup.execute(qsTr("Clipboard is empty"), function() {})
            return
        }
        var formatted = BlockHtmlUtils.formatPasteWithPrefix(clipText, prefix)
        var ctx = _editorCtx()
        EH.pasteSpecial(ctx, formatted, BlockHtmlUtils)
        _syncFromCtx(ctx)
    }

    function applyPrefixToActiveEditor(prefix, multiLineList) {
        var ctx = _editorCtx()
        EH.applyPrefix(ctx, prefix, BlockHtmlUtils)
        _syncFromCtx(ctx)
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
        var dialog = pageStack.push(Qt.resolvedUrl("../dialogs/PageLinkDialog.qml"), {
            selectedText: sel
        })
        dialog.accepted.connect(function() {
            var ctx = _editorCtx()
            EH.insertLink(ctx, dialog.formattedLink)
            _syncFromCtx(ctx)
        })
    }

    function changeActiveEditorListLevel(delta) {
        var ctx = _editorCtx()
        EH.changeListLevel(ctx, delta, BlockHtmlUtils)
        _syncFromCtx(ctx)
    }

    function moveActiveEditorLines(direction) {
        var ctx = _editorCtx()
        EH.moveLines(ctx, direction, BlockHtmlUtils)
        _syncFromCtx(ctx)
    }

    function applyJournalBlockPrefix(prefix, multiLineList) {
        applyPrefixToActiveEditor(prefix, multiLineList)
    }

    function isJournalEditingDirty() {
        if (mainPage.isAddingJournalBlock) {
            return (mainPage.newJournalBlockText || "").trim().length > 0
        }
        if (mainPage.editingJournalBlockIndex >= 0) {
            return mainPage.editingCurrentText !== mainPage.editingRawText
        }
        return false
    }

    function requestCancelJournalEditing() {
        if (mainPage.isJournalEditingDirty()) {
            mainPage.showJournalDiscardConfirmation = true
        } else {
            mainPage.showJournalDiscardConfirmation = false
            if (mainPage.isAddingJournalBlock) {
                mainPage.cancelNewJournalBlock()
            } else {
                mainPage.cancelCurrentJournalEditing()
            }
        }
    }

    function saveCurrentEditingJournalBlock() {
        mainPage.showJournalDiscardConfirmation = false
        if (editingJournalBlockIndex >= 0) {
            bridge.save_journal_block(editingJournalBlockIndex, editingCurrentText)
            editingJournalBlockIndex = -1
            editingRawText = ""
            editingCurrentText = ""
            currentEditorTextArea = null
        }
    }

    function cancelCurrentJournalEditing() {
        mainPage.showJournalDiscardConfirmation = false
        editingJournalBlockIndex = -1
        editingRawText = ""
        editingCurrentText = ""
        currentEditorTextArea = null
    }

    function startAddingJournalBlock() {
        mainPage.showJournalDiscardConfirmation = false
        if (editingJournalBlockIndex >= 0) {
            saveCurrentEditingJournalBlock()
        }
        newJournalBlockText = ""
        isAddingJournalBlock = true
        currentEditorTextArea = journalInlineNewTextArea
        journalInlineNewTextArea.forceActiveFocus()
    }

    function saveNewJournalBlock() {
        mainPage.showJournalDiscardConfirmation = false
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
        mainPage.showJournalDiscardConfirmation = false
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
                text: qsTr("About Notes Plus")
                onClicked: {
                    pageStack.push(Qt.resolvedUrl("AboutPage.qml"))
                }
            }
            MenuItem {
                text: qsTr("Share Server URL")
                visible: bridge.web_server_running
                onClicked: {
                    Clipboard.text = bridge.web_server_url
                    app.notification.text = qsTr("Server URL copied to clipboard")
                    app.notification.show()
                }
            }
            MenuItem {
                text: qsTr("Backup & Export")
                onClicked: {
                    pageStack.push(Qt.resolvedUrl("BackupPage.qml"))
                }
            }
            MenuItem {
                text: qsTr("Settings")
                onClicked: {
                    pageStack.push(Qt.resolvedUrl("SettingsPage.qml"))
                }
            }
            MenuItem {
                text: qsTr("AI Import")
                visible: (typeof app !== "undefined" && app && app.aiEnabled !== undefined) ? app.aiEnabled : true
                onClicked: {
                    pageStack.push(Qt.resolvedUrl("AiImportPage.qml"))
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
                    var dialog = pageStack.push(Qt.resolvedUrl("../dialogs/NewGroupDialog.qml"))
                    dialog.accepted.connect(function() {
                        if (dialog.groupName.length > 0) {
                            if (bridge.create_group("", dialog.groupName)) {
                                if (dialog.noteSort && dialog.noteSort !== "newest") {
                                    bridge.set_group_note_sort(dialog.groupName, dialog.noteSort)
                                }
                            }
                        }
                    })
                }
            }
            MenuItem {
                text: qsTr("New Note")
                onClicked: {
                    var dialog = pageStack.push(Qt.resolvedUrl("../dialogs/NewPageDialog.qml"))
                    dialog.accepted.connect(function() {
                        if (dialog.pageName.length > 0) {
                            bridge.create_page(dialog.pageName, dialog.selectedColor)
                        }
                    })
                }
            }
            MenuItem {
                text: qsTr("Journal")
                visible: (typeof app !== "undefined" && app && app.journalEnabled !== undefined) ? app.journalEnabled : true
                onClicked: {
                    pageStack.push(Qt.resolvedUrl("PageView.qml"), {
                        initialTargetPage: "journal",
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
                title: "Notes Plus"
                description: bridge.web_server_running ? bridge.web_server_url : ""
            }

            Item {
                width: parent.width
                height: searchField.height

                SearchField {
                    id: searchField
                    anchors.left: parent.left
                    anchors.right: searchMicItem.visible ? searchMicItem.left : parent.right
                    placeholderText: qsTr("Search your notes...")
                    onTextChanged: {
                        if (text.length > 0) {
                            searchDebounce.restart()
                        } else {
                            searchDebounce.stop()
                            bridge.search("")
                        }
                    }
                }

                Timer {
                    id: searchDebounce
                    interval: 300
                    onTriggered: {
                        if (searchField.text.length > 0) {
                            bridge.do_search(searchField.text)
                        }
                    }
                }

                Item {
                    id: searchMicItem
                    anchors.right: parent.right
                    anchors.rightMargin: Theme.horizontalPageMargin
                    anchors.verticalCenter: parent.verticalCenter
                    width: visible ? Theme.itemSizeExtraSmall : 0
                    height: Theme.itemSizeExtraSmall
                    visible: typeof app === "undefined" || !app || app.sttEnabled !== false

                    readonly property bool isRecording: typeof speechBridge !== "undefined" && speechBridge && speechBridge.is_recording
                    readonly property real liveAudioLevel: (isRecording && typeof speechBridge !== "undefined" && speechBridge) ? speechBridge.audio_level : 0.0

                    Rectangle {
                        anchors.centerIn: parent
                        width: Math.min(parent.width, Theme.iconSizeSmall + Theme.paddingSmall + Math.round(searchMicItem.liveAudioLevel * 20))
                        height: width
                        radius: width / 2
                        color: (searchMicItem.liveAudioLevel > 0.06) ? Theme.highlightColor : Theme.secondaryColor
                        opacity: searchMicItem.isRecording ? Math.min(0.85, 0.25 + searchMicItem.liveAudioLevel * 0.6) : 0.0
                        visible: searchMicItem.isRecording

                        Behavior on width { NumberAnimation { duration: 60 } }
                        Behavior on height { NumberAnimation { duration: 60 } }
                        Behavior on opacity { NumberAnimation { duration: 60 } }
                    }

                    IconButton {
                        id: searchMicBtn
                        anchors.centerIn: parent
                        icon.source: searchMicItem.isRecording ? "image://theme/icon-m-clear" : "image://theme/icon-m-mic"
                        highlighted: searchMicItem.isRecording
                        onClicked: {
                            voiceController.toggleMic(searchField)
                        }
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
                remorsePopupRef: remorsePopup
                visible: searchField.text.length > 0 && parsedSearchResults.length > 0
                onItemClicked: function(itemData, itemIndex) {
                    var target = itemData.full_path || itemData.name
                    pageStack.push(Qt.resolvedUrl("PageView.qml"), {
                        initialTargetPage: target,
                        pageName: itemData.name,
                        findInPageTerm: searchField.text
                    })
                    bridge.load_page(target)
                }
            }

            // Search loading indicator
            Label {
                x: Theme.horizontalPageMargin
                width: parent.width - Theme.horizontalPageMargin * 2
                visible: searchField.text.length > 0 && parsedSearchResults.length === 0 && (searchDebounce.running || bridge.search_loading)
                text: qsTr("Searching...")
                color: Theme.highlightColor
                font.pixelSize: Theme.fontSizeSmall
                wrapMode: Text.Wrap
            }

            // No search results empty state
            Label {
                x: Theme.horizontalPageMargin
                width: parent.width - Theme.horizontalPageMargin * 2
                visible: searchField.text.length > 0 && parsedSearchResults.length === 0 && !searchDebounce.running && !bridge.search_loading
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
                            mainPage.showJournalDiscardConfirmation = false
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
                                if (mainPage.showJournalDiscardConfirmation) {
                                    mainPage.showJournalDiscardConfirmation = false
                                }
                            }
                        }

                        onSaveRequested: function(idx, newRaw) {
                            mainPage.showJournalDiscardConfirmation = false
                            bridge.save_journal_block(idx, newRaw)
                            mainPage.editingJournalBlockIndex = -1
                            mainPage.editingRawText = ""
                            mainPage.editingCurrentText = ""
                            mainPage.currentEditorTextArea = null
                        }

                        onCancelEditRequested: function() {
                            mainPage.requestCancelJournalEditing()
                        }

                        onCheckboxToggled: function(idx, itemPath) {
                            var relativePath = (itemPath.length > String(idx).length + 1) ? itemPath.substring(String(idx).length + 1) : ""
                            bridge.toggle_journal_checkbox(idx, relativePath)
                        }

                        onXrefActivated: function(target) {
                            var hashIdx = target.indexOf("#")
                            var pagePart = (hashIdx >= 0) ? target.substring(0, hashIdx) : target
                            var anchor = (hashIdx >= 0) ? target.substring(hashIdx + 1) : ""
                            if (pagePart.indexOf(".adoc") === pagePart.length - 5 && pagePart.length >= 5) {
                                pagePart = pagePart.substring(0, pagePart.length - 5)
                            }
                            pageStack.push(Qt.resolvedUrl("PageView.qml"), {
                                initialTargetPage: pagePart,
                                initialAnchor: anchor
                            })
                            bridge.load_page(pagePart)
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
                        Keys.onPressed: function(event) {
                            BlockHtmlUtils.handleEditorKeyPress(event, journalInlineNewTextArea)
                        }
                        onTextChanged: {
                            if (mainPage.isAddingJournalBlock) {
                                mainPage.newJournalBlockText = text
                                if (mainPage.showJournalDiscardConfirmation) {
                                    mainPage.showJournalDiscardConfirmation = false
                                }
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
                            initialTargetPage: "journal",
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
                    text: qsTr("Welcome to Notes Plus")
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
        showVoiceInput: false
        anchors.right: parent.right
        anchors.rightMargin: Theme.paddingMedium
        anchors.verticalCenter: parent.verticalCenter
        visible: (typeof app !== "undefined" && app && app.journalEnabled !== undefined ? app.journalEnabled : true) && (mainPage.editingJournalBlockIndex >= 0 || mainPage.isAddingJournalBlock)
        onAccepted: {
            mainPage.showJournalDiscardConfirmation = false
            if (mainPage.isAddingJournalBlock) {
                mainPage.saveNewJournalBlock()
            } else {
                mainPage.saveCurrentEditingJournalBlock()
            }
        }
        onCanceled: {
            mainPage.requestCancelJournalEditing()
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
        onElementPickerRequested: {
            mainPage.openElementPicker()
        }
        onIndentRequested: {
            mainPage.changeActiveEditorListLevel(1)
        }
        onOutdentRequested: {
            mainPage.changeActiveEditorListLevel(-1)
        }
        onMoveUpRequested: {
            mainPage.moveActiveEditorLines(-1)
        }
        onMoveDownRequested: {
            mainPage.moveActiveEditorLines(1)
        }
    }

    DiscardConfirmationBanner {
        id: journalDiscardBanner
        anchors.bottom: parent.bottom
        anchors.bottomMargin: (Qt.inputMethod.visible ? Math.min(Qt.inputMethod.keyboardRectangle.height, Screen.height * 0.5) : 0) + Theme.paddingLarge
        open: mainPage.showJournalDiscardConfirmation
        onDiscardConfirmed: {
            mainPage.showJournalDiscardConfirmation = false
            if (mainPage.isAddingJournalBlock) {
                mainPage.cancelNewJournalBlock()
            } else {
                mainPage.cancelCurrentJournalEditing()
            }
        }
        onKeepEditing: {
            mainPage.showJournalDiscardConfirmation = false
            mainPage.refocusActiveEditor()
        }
    }

    VoiceInputController {
        id: voiceController
        speechBridgeRef: speechBridge
        textFilter: function(text) {
            if (typeof text !== "string") return text
            return text.replace(/[.!?]+$/, "").trim()
        }
    }

    VoiceFeedbackBanner {
        id: voiceFeedbackBanner
        controller: voiceController
    }
}
