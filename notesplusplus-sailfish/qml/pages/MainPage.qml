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

    function applyJournalBlockPrefix(prefix, multiLineList) {
        var txt = editingCurrentText || ""
        var newText = BlockHtmlUtils.stripAndApplyPrefix(txt, prefix)
        editingRawText = newText
        editingCurrentText = newText
    }

    function saveCurrentEditingJournalBlock() {
        if (editingJournalBlockIndex >= 0) {
            bridge.save_journal_block(editingJournalBlockIndex, editingCurrentText)
            editingJournalBlockIndex = -1
            editingRawText = ""
            editingCurrentText = ""
        }
    }

    function cancelCurrentJournalEditing() {
        editingJournalBlockIndex = -1
        editingRawText = ""
        editingCurrentText = ""
    }

    function startAddingJournalBlock() {
        if (editingJournalBlockIndex >= 0) {
            saveCurrentEditingJournalBlock()
        }
        newJournalBlockText = ""
        isAddingJournalBlock = true
    }

    function saveNewJournalBlock() {
        if (isAddingJournalBlock) {
            var trimmed = (newJournalBlockText || "").trim()
            if (trimmed.length > 0) {
                bridge.append_to_journal(trimmed, false)
            }
            isAddingJournalBlock = false
            newJournalBlockText = ""
        }
    }

    function cancelNewJournalBlock() {
        isAddingJournalBlock = false
        newJournalBlockText = ""
    }

    function applyNewJournalBlockPrefix(prefix, multiLineList) {
        newJournalBlockText = BlockHtmlUtils.stripAndApplyPrefix(newJournalBlockText || "", prefix)
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
                text: qsTr("New Page")
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
                            remorsePopup.execute(qsTr("Copied: ") + url, function() {}, 3000)
                        }
                    }
                }
            }

            SearchField {
                id: searchField
                width: parent.width
                placeholderText: qsTr("Search pages...")
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
                    pageStack.push(Qt.resolvedUrl("PageView.qml"), {
                        pageName: itemData.name,
                        searchTerm: searchField.text
                    })
                    bridge.load_page(itemData.name)
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

            // Recent pages section
            SectionHeader {
                text: qsTr("Recent Pages")
                visible: searchField.text.length === 0
            }

            NoteCardGrid {
                id: recentGrid
                width: parent.width
                isPortraitOrientation: isPortrait
                model: searchField.text.length === 0 ? parsedRecentPages : []
                visible: searchField.text.length === 0
                onItemClicked: function(itemData, itemIndex) {
                    pageStack.push(Qt.resolvedUrl("PageView.qml"), {
                        pageName: itemData.name
                    })
                    bridge.load_page(itemData.name)
                }
            }

            // Empty wiki guidance
            Column {
                width: parent.width - Theme.horizontalPageMargin * 2
                anchors.horizontalCenter: parent.horizontalCenter
                spacing: Theme.paddingMedium
                visible: searchField.text.length === 0 && parsedRecentPages.length === 0

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
            if (mainPage.isAddingJournalBlock) {
                mainPage.applyNewJournalBlockPrefix(prefix, multiLine)
            } else {
                mainPage.applyJournalBlockPrefix(prefix, multiLine)
            }
        }
        onLinkRequested: {
            var dialog = pageStack.push(Qt.resolvedUrl("PageLinkDialog.qml"), {
                selectedText: ""
            })
            dialog.accepted.connect(function() {
                var link = dialog.formattedLink
                if (!link) return
                if (mainPage.isAddingJournalBlock) {
                    mainPage.newJournalBlockText = (mainPage.newJournalBlockText && mainPage.newJournalBlockText.length > 0 ? mainPage.newJournalBlockText + " " : "") + link
                } else if (mainPage.editingJournalBlockIndex >= 0) {
                    var cur = mainPage.editingCurrentText || ""
                    var updated = (cur.length > 0 ? cur + " " : "") + link
                    mainPage.editingRawText = updated
                    mainPage.editingCurrentText = updated
                }
            })
        }
        onPasteRequested: {
            var clipText = Clipboard.text
            if (!clipText || clipText.length === 0) {
                remorsePopup.execute(qsTr("Clipboard is empty"), function() {})
                return
            }
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

    RemorsePopup { id: remorsePopup }
}
