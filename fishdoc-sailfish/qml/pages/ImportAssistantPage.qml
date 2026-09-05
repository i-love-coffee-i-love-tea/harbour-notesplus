import QtQuick 2.0
import Sailfish.Silica 1.0
import harbour.fishdoc 1.0
import "../components"

Page {
    id: importPage

    property bool showUrlInput: false
    property bool showFileInput: false

    function applyConfig() {
        if (typeof agentBridge !== "undefined" && agentBridge) {
            agentBridge.configure(
                app.aiProvider || "ollama",
                app.aiEndpoint || "http://192.168.1.1:11434",
                app.aiModel || "llama3.2",
                app.aiApiKey || "",
                app.aiTimeout || 90,
                app.aiAutoAllowRead !== undefined ? app.aiAutoAllowRead : true,
                app.aiAutoAllowCreate !== undefined ? app.aiAutoAllowCreate : true,
                app.aiRequireConfirmEdit !== undefined ? app.aiRequireConfirmEdit : true
            )
        }
    }

    onStatusChanged: {
        if (status === PageStatus.Active) {
            applyConfig()
        }
    }

    Connections {
        target: app
        onAiProviderChanged: importPage.applyConfig()
        onAiEndpointChanged: importPage.applyConfig()
        onAiModelChanged: importPage.applyConfig()
        onAiApiKeyChanged: importPage.applyConfig()
        onAiTimeoutChanged: importPage.applyConfig()
        onAiAutoAllowReadChanged: importPage.applyConfig()
        onAiAutoAllowCreateChanged: importPage.applyConfig()
        onAiRequireConfirmEditChanged: importPage.applyConfig()
    }

    AgentBridge {
        id: agentBridge

        Component.onCompleted: {
            importPage.applyConfig()
        }

        onError_occurred: {
            remorsePopup.execute("AI Error: " + message, function() {})
        }

        onResponse_finished: {
            bridge.load_main_page_data()
        }

        onUndo_completed: {
            bridge.load_main_page_data()
            remorsePopup.execute(message, function() {})
        }
    }

    Timer {
        id: pollTimer
        interval: 120
        running: agentBridge.agent_busy
        repeat: true
        onTriggered: {
            agentBridge.poll_worker()
        }
    }

    RemorsePopup {
        id: remorsePopup
    }

    SilicaFlickable {
        id: flickable
        anchors.fill: parent
        contentHeight: contentCol.height + Theme.paddingLarge

        PullDownMenu {
            MenuItem {
                text: "Settings"
                onClicked: pageStack.push(Qt.resolvedUrl("SettingsPage.qml"))
            }
            MenuItem {
                text: "Paste from Clipboard"
                onClicked: {
                    if (Clipboard.text && Clipboard.text.length > 0) {
                        sourceTextArea.text = Clipboard.text
                    } else {
                        remorsePopup.execute("Clipboard is empty", function() {})
                    }
                }
            }
            MenuItem {
                text: "Clear Fields"
                onClicked: {
                    sourceTextArea.text = ""
                    titleField.text = ""
                    urlField.text = ""
                    filePathField.text = ""
                    customPromptField.text = ""
                }
            }
        }

        Column {
            id: contentCol
            width: parent.width
            spacing: Theme.paddingMedium

            PageHeader {
                title: "Import Assistant"
                description: "Convert text from external sources to AsciiDoc"
            }

            // Source Text Section
            SectionHeader {
                text: "Source Content"
            }

            Row {
                anchors.horizontalCenter: parent.horizontalCenter
                spacing: Theme.paddingSmall

                Button {
                    text: "📋 Paste Clipboard"
                    preferredWidth: Theme.buttonWidthExtraSmall
                    enabled: !agentBridge.agent_busy
                    onClicked: {
                        if (Clipboard.text && Clipboard.text.length > 0) {
                            sourceTextArea.text = Clipboard.text
                        } else {
                            remorsePopup.execute("Clipboard is empty", function() {})
                        }
                    }
                }

                Button {
                    text: "🌐 Fetch URL"
                    preferredWidth: Theme.buttonWidthExtraSmall
                    enabled: !agentBridge.agent_busy
                    onClicked: {
                        importPage.showUrlInput = !importPage.showUrlInput
                    }
                }

                Button {
                    text: "📁 Local File"
                    preferredWidth: Theme.buttonWidthExtraSmall
                    enabled: !agentBridge.agent_busy
                    onClicked: {
                        importPage.showFileInput = !importPage.showFileInput
                    }
                }
            }

            // URL Input Row (expandable)
            Item {
                width: parent.width - Theme.horizontalPageMargin * 2
                height: importPage.showUrlInput ? (urlRow.height + Theme.paddingSmall) : 0
                anchors.horizontalCenter: parent.horizontalCenter
                visible: importPage.showUrlInput
                clip: true

                Behavior on height { NumberAnimation { duration: 150 } }

                Row {
                    id: urlRow
                    width: parent.width
                    spacing: Theme.paddingSmall

                    TextField {
                        id: urlField
                        width: parent.width - fetchBtn.width - Theme.paddingSmall
                        placeholderText: "https://example.com/article"
                        label: "Web Page URL"
                        inputMethodHints: Qt.ImhUrlCharactersOnly
                        EnterKey.onClicked: fetchBtn.clicked()
                    }

                    Button {
                        id: fetchBtn
                        text: "Fetch"
                        preferredWidth: Theme.buttonWidthExtraSmall
                        enabled: urlField.text.trim().length > 0 && !agentBridge.agent_busy
                        anchors.verticalCenter: urlField.verticalCenter
                        onClicked: {
                            var content = agentBridge.fetch_url_content(urlField.text)
                            if (content.indexOf("Error") === 0) {
                                remorsePopup.execute(content, function() {})
                            } else {
                                sourceTextArea.text = content
                                importPage.showUrlInput = false
                            }
                        }
                    }
                }
            }

            // File Path Input Row (expandable)
            Item {
                width: parent.width - Theme.horizontalPageMargin * 2
                height: importPage.showFileInput ? (fileRow.height + Theme.paddingSmall) : 0
                anchors.horizontalCenter: parent.horizontalCenter
                visible: importPage.showFileInput
                clip: true

                Behavior on height { NumberAnimation { duration: 150 } }

                Row {
                    id: fileRow
                    width: parent.width
                    spacing: Theme.paddingSmall

                    TextField {
                        id: filePathField
                        width: parent.width - readFileBtn.width - Theme.paddingSmall
                        placeholderText: "~/Documents/notes.txt or markdown.md"
                        label: "Local File Path"
                        EnterKey.onClicked: readFileBtn.clicked()
                    }

                    Button {
                        id: readFileBtn
                        text: "Load"
                        preferredWidth: Theme.buttonWidthExtraSmall
                        enabled: filePathField.text.trim().length > 0 && !agentBridge.agent_busy
                        anchors.verticalCenter: filePathField.verticalCenter
                        onClicked: {
                            var content = agentBridge.read_local_file(filePathField.text)
                            if (content.indexOf("Error") === 0) {
                                remorsePopup.execute(content, function() {})
                            } else {
                                sourceTextArea.text = content
                                importPage.showFileInput = false
                            }
                        }
                    }
                }
            }

            TextArea {
                id: sourceTextArea
                width: parent.width - Theme.horizontalPageMargin * 2
                anchors.horizontalCenter: parent.horizontalCenter
                placeholderText: "Paste external text, Markdown, notes, emails, transcripts, or articles here..."
                label: "Source Text"
                height: Math.max(Theme.itemSizeLarge * 2, implicitHeight)
            }

            // Options Section
            SectionHeader {
                text: "Import Options"
            }

            TextField {
                id: titleField
                width: parent.width - Theme.horizontalPageMargin * 2
                anchors.horizontalCenter: parent.horizontalCenter
                label: "Note Title (Optional)"
                placeholderText: "Auto-detected if left empty"
                EnterKey.iconSource: "image://theme/icon-m-enter-close"
                EnterKey.onClicked: focus = false
            }

            ComboBox {
                id: modeComboBox
                width: parent.width
                label: "Conversion Mode"
                currentIndex: 0

                menu: ContextMenu {
                    MenuItem { text: "Full Document (Complete AsciiDoc)" }
                    MenuItem { text: "Summarize & Structure" }
                    MenuItem { text: "Extract Action Items / Checklists" }
                }
            }

            TextField {
                id: customPromptField
                width: parent.width - Theme.horizontalPageMargin * 2
                anchors.horizontalCenter: parent.horizontalCenter
                label: "Custom Instructions (Optional)"
                placeholderText: "e.g. Add syntax highlighting, keep concise"
                EnterKey.iconSource: "image://theme/icon-m-enter-close"
                EnterKey.onClicked: focus = false
            }

            // Action Button & Progress
            Item {
                width: parent.width
                height: Theme.itemSizeMedium

                Button {
                    id: convertBtn
                    anchors.centerIn: parent
                    text: agentBridge.agent_busy ? "Converting & Importing..." : "🚀 Convert & Import as Note"
                    enabled: !agentBridge.agent_busy && sourceTextArea.text.trim().length > 0
                    onClicked: {
                        importPage.applyConfig()
                        var mode = "convert_full"
                        if (modeComboBox.currentIndex === 1) mode = "summarize"
                        else if (modeComboBox.currentIndex === 2) mode = "action_items"

                        agentBridge.import_text(
                            sourceTextArea.text,
                            titleField.text,
                            mode,
                            customPromptField.text
                        )
                    }
                }
            }

            BusyIndicator {
                anchors.horizontalCenter: parent.horizontalCenter
                size: BusyIndicatorSize.Medium
                running: agentBridge.agent_busy
                visible: agentBridge.agent_busy
            }

            // Undo Banner
            UndoBanner {
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.margins: Theme.horizontalPageMargin
                visible: agentBridge.can_undo
                onUndoTriggered: {
                    agentBridge.undo_last_action()
                }
            }

            // Pending Confirmation Card (for safe inspection if edit was requested)
            ConfirmationCard {
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.margins: Theme.horizontalPageMargin
                visible: agentBridge.has_pending_action
                actionData: {
                    try {
                        return agentBridge.pending_action_json.length > 0 ? JSON.parse(agentBridge.pending_action_json) : null
                    } catch (e) {
                        return null
                    }
                }
                onConfirmed: function(approved) {
                    agentBridge.confirm_action(approved)
                }
            }

            // Result Card when a note is created
            Rectangle {
                id: resultCard
                width: parent.width - Theme.horizontalPageMargin * 2
                anchors.horizontalCenter: parent.horizontalCenter
                height: resultColumn.height + Theme.paddingLarge * 2
                color: Theme.rgba(Theme.highlightBackgroundColor, 0.1)
                radius: Theme.paddingSmall
                border.color: Theme.rgba(Theme.highlightColor, 0.3)
                border.width: 1
                visible: agentBridge.last_created_note.length > 0 && !agentBridge.agent_busy

                Column {
                    id: resultColumn
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: parent.top
                    anchors.margins: Theme.paddingLarge
                    spacing: Theme.paddingMedium

                    Row {
                        spacing: Theme.paddingSmall
                        anchors.horizontalCenter: parent.horizontalCenter

                        Label {
                            text: "✓"
                            color: "#4cd964"
                            font.pixelSize: Theme.fontSizeLarge
                            font.bold: true
                            anchors.verticalCenter: parent.verticalCenter
                        }

                        Label {
                            text: "Import Completed"
                            color: Theme.highlightColor
                            font.pixelSize: Theme.fontSizeMedium
                            font.bold: true
                            anchors.verticalCenter: parent.verticalCenter
                        }
                    }

                    Label {
                        text: "Created Note: " + agentBridge.last_created_note
                        color: Theme.primaryColor
                        font.pixelSize: Theme.fontSizeMedium
                        font.bold: true
                        horizontalAlignment: Text.AlignHCenter
                        anchors.horizontalCenter: parent.horizontalCenter
                        wrapMode: Text.Wrap
                        width: parent.width
                    }

                    Button {
                        text: "📖 Open Created Note"
                        anchors.horizontalCenter: parent.horizontalCenter
                        preferredWidth: Theme.buttonWidthMedium
                        onClicked: {
                            var noteTitle = agentBridge.last_created_note
                            pageStack.push(Qt.resolvedUrl("PageView.qml"), {
                                pageName: noteTitle
                            })
                            bridge.load_page(noteTitle)
                        }
                    }
                }
            }

            // Conversational Output Timeline
            Column {
                width: parent.width - Theme.horizontalPageMargin * 2
                anchors.horizontalCenter: parent.horizontalCenter
                spacing: Theme.paddingMedium
                visible: !agentBridge.agent_busy

                Repeater {
                    model: {
                        try {
                            var list = JSON.parse(agentBridge.messages_json)
                            return list.filter(function(m) {
                                return m.role !== "system" && m.role !== "tool"
                            })
                        } catch (e) {
                            return []
                        }
                    }

                    delegate: Rectangle {
                        width: parent.width
                        height: msgCol.height + Theme.paddingMedium * 2
                        color: modelData.role === "assistant" ? Theme.rgba(Theme.highlightBackgroundColor, 0.08) : Theme.rgba(Theme.primaryColor, 0.04)
                        radius: Theme.paddingSmall
                        border.color: modelData.role === "assistant" ? Theme.rgba(Theme.highlightColor, 0.2) : "transparent"
                        border.width: 1

                        Column {
                            id: msgCol
                            anchors.left: parent.left
                            anchors.right: parent.right
                            anchors.top: parent.top
                            anchors.margins: Theme.paddingMedium
                            spacing: Theme.paddingSmall

                            Label {
                                text: modelData.role === "assistant" ? "🤖 Assistant Summary" : "👤 Import Request"
                                color: Theme.highlightColor
                                font.pixelSize: Theme.fontSizeExtraSmall
                                font.bold: true
                            }

                            Label {
                                text: modelData.content || ""
                                color: Theme.primaryColor
                                font.pixelSize: Theme.fontSizeSmall
                                wrapMode: Text.Wrap
                                width: parent.width
                            }
                        }
                    }
                }
            }
        }
    }
}
