import QtQuick 2.0
import Sailfish.Silica 1.0
import harbour.notesplusplus 1.0
import "../components"

Page {
    id: assistantPage
    allowedOrientations: Orientation.All

    property string contextFilename: ""
    property string contextContent: ""
    property int currentTab: 0

    property bool hasContextOrInput: (contextContent.length > 0) || (promptBar && promptBar.text && promptBar.text.trim().length > 0)

    // Speech capture state comes from SpeechBridge (native 16 kHz mono WAV recorder).
    property bool isSpeechRecording: typeof speechBridge !== "undefined" && speechBridge && speechBridge.is_recording
    property bool isSpeechTranscribing: typeof speechBridge !== "undefined" && speechBridge && speechBridge.is_transcribing
    property real liveAudioLevel: (typeof speechBridge !== "undefined" && speechBridge && assistantPage.isSpeechRecording) ? speechBridge.audio_level : 0.0
    property var liveWaveform: {
        if (typeof speechBridge !== "undefined" && speechBridge && speechBridge.waveform_json && assistantPage.isSpeechRecording) {
            try {
                return JSON.parse(speechBridge.waveform_json) || [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
            } catch (e) {
                return [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
            }
        }
        return [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
    }

    function scrollToBottom() {
        scrollTimer.restart()
    }

    function cancelActiveRecording() {
        if (typeof speechBridge !== "undefined" && speechBridge && speechBridge.is_recording) {
            speechBridge.cancel_recording()
        }
    }

    function handleMicClick() {
        if (typeof speechBridge === "undefined" || !speechBridge) {
            remorsePopup.execute(qsTr("Speech recognition is unavailable."), function() {})
            return
        }
        if (speechBridge.is_recording) {
            speechBridge.stop_recording_and_transcribe()
        } else {
            if (!speechBridge.has_installed_models) {
                remorsePopup.execute(qsTr("No speech model installed. Please download a model."), function() {})
                pageStack.push(Qt.resolvedUrl("ModelDownloadDialog.qml"))
                return
            }
            if (!speechBridge.start_recording()) {
                return
            }
        }
    }

    Timer {
        id: scrollTimer
        interval: 80
        repeat: false
        onTriggered: {
            if (flickable.contentHeight > flickable.height) {
                flickable.contentY = flickable.contentHeight - flickable.height
            }
        }
    }

    function applyConfig() {
        if (typeof agentBridge !== "undefined" && agentBridge) {
            agentBridge.configure(
                app.aiProvider || "ollama",
                app.aiEndpoint || app.defaultAiEndpoint,
                app.aiModel || "llama3.2",
                app.aiApiKey || "",
                app.aiTimeout || 90,
                app.aiAutoAllowRead !== undefined ? app.aiAutoAllowRead : true,
                app.aiAutoAllowCreate !== undefined ? app.aiAutoAllowCreate : true,
                app.aiRequireConfirmEdit !== undefined ? app.aiRequireConfirmEdit : true,
                app.aiAllowSelfSigned !== undefined ? app.aiAllowSelfSigned : false,
                app.aiAllowFetchUrl !== undefined ? app.aiAllowFetchUrl : true
            )
        }
    }

    onStatusChanged: {
        if (status === PageStatus.Active) {
            applyConfig()
        } else if (status === PageStatus.Deactivating || status === PageStatus.Inactive) {
            // Leaving the page cancels capture; do not auto-transcribe mid-flight audio.
            cancelActiveRecording()
        }
    }

    // Dsnote-style property binding: QML re-evaluates this when the NOTIFY
    // signal (transcription_completed) fires, so no signal parameter needed.
    property string pendingTranscription: (typeof speechBridge !== "undefined" && speechBridge) ? speechBridge.last_transcription : ""
    onPendingTranscriptionChanged: {
        if (assistantPage.status !== PageStatus.Active) return
        var trans = pendingTranscription
        if (trans && trans.trim().length > 0) {
            var clean = trans.trim()
            if (currentTab === 0) {
                if (promptBar.text.length > 0) {
                    promptBar.text = promptBar.text + " " + clean
                } else {
                    promptBar.text = clean
                }
            } else {
                if (importTab.sourceText.length > 0) {
                    importTab.sourceText = importTab.sourceText + " " + clean
                } else {
                    importTab.sourceText = clean
                }
            }
            assistantPage.scrollToBottom()
        }
    }

    Connections {
        target: (typeof speechBridge !== "undefined" && speechBridge) ? speechBridge : null
        onError_occurred: {
            var errMsg = (typeof message !== "undefined" && message) ? message :
                         ((typeof speechBridge !== "undefined" && speechBridge && speechBridge.error_message) ? speechBridge.error_message : "")
            if (errMsg && errMsg.length > 0) {
                remorsePopup.execute(errMsg, function() {})
            }
        }
    }

    Connections {
        target: agentBridge

        onSession_initialized: {
            if (contextFilename.length > 0) {
                agentBridge.reset_session(contextFilename, contextContent, "")
            }
        }

        onError_occurred: {
            remorsePopup.execute("AI Error: " + message, function() {})
        }

        onMessages_changed: {
            assistantPage.scrollToBottom()
        }

        onBusy_changed: {
            if (agent_busy) {
                assistantPage.scrollToBottom()
            }
        }

        onStreaming_text_changed: {
            assistantPage.scrollToBottom()
        }

        onPending_action_changed: {
            if (has_pending_action) {
                assistantPage.scrollToBottom()
            }
        }

        onResponse_finished: {
            bridge.load_main_page_data()
            assistantPage.scrollToBottom()
        }

        onFetch_completed: {
            importTab.sourceText = result
            importTab.showUrlInput = false
            importTab.showFileInput = false
        }

        onFetch_error: {
            remorsePopup.execute(message, function() {})
        }

        onUndo_completed: {
            bridge.load_main_page_data()
            remorsePopup.execute(message, function() {})
        }
    }

    Timer {
        id: pollTimer
        interval: 50
        running: agentBridge.agent_busy || agentBridge.is_fetching
        repeat: true
        onTriggered: {
            agentBridge.poll_worker()
        }
    }

    SilicaFlickable {
        id: flickable
        anchors.fill: parent
        contentHeight: mainColumn.height + Theme.paddingLarge

        PullDownMenu {
            MenuItem {
                text: qsTr("Settings")
                onClicked: pageStack.push(Qt.resolvedUrl("SettingsPage.qml"))
            }

            MenuItem {
                text: qsTr("Manage Speech Models")
                visible: currentTab === 0
                onClicked: pageStack.push(Qt.resolvedUrl("ModelDownloadDialog.qml"))
            }

            // Chat-specific actions
            MenuItem {
                text: qsTr("Paste Clipboard Context")
                visible: currentTab === 0
                onClicked: {
                    if (Clipboard.text && Clipboard.text.length > 0) {
                        agentBridge.reset_session(contextFilename, contextContent, Clipboard.text)
                        promptBar.text = qsTr("Please analyze the clipboard content.")
                    } else {
                        remorsePopup.execute(qsTr("Clipboard is empty"), function() {})
                    }
                }
            }
            MenuItem {
                text: qsTr("Clear Conversation")
                visible: currentTab === 0
                onClicked: {
                    agentBridge.reset_session(contextFilename, contextContent, "")
                }
            }

            // Import-specific actions
            MenuItem {
                text: qsTr("Paste from Clipboard")
                visible: currentTab === 1
                onClicked: {
                    if (Clipboard.text && Clipboard.text.length > 0) {
                        importTab.sourceText = Clipboard.text
                    } else {
                        remorsePopup.execute(qsTr("Clipboard is empty"), function() {})
                    }
                }
            }
            MenuItem {
                text: qsTr("Clear Import Fields")
                visible: currentTab === 1
                onClicked: {
                    importTab.sourceText = ""
                    importTab.noteTitle = ""
                    importTab.urlText = ""
                    importTab.filePathText = ""
                    importTab.customPrompt = ""
                }
            }
        }

        Column {
            id: mainColumn
            width: parent.width
            spacing: Theme.paddingMedium

            PageHeader {
                title: qsTr("AI Assistant")
                description: currentTab === 0 ? (contextFilename.length > 0 ? (qsTr("Context: ") + contextFilename) : "") : qsTr("Convert external sources to AsciiDoc")
            }

            // Tab Bar: Chat vs Import
            Row {
                width: parent.width - Theme.horizontalPageMargin * 2
                anchors.horizontalCenter: parent.horizontalCenter
                spacing: Theme.paddingSmall

                Button {
                    text: qsTr("Chat")
                    preferredWidth: (parent.width - Theme.paddingSmall) / 2
                    color: currentTab === 0 ? Theme.highlightColor : Theme.primaryColor
                    backgroundColor: currentTab === 0 ? Theme.rgba(Theme.highlightBackgroundColor, 0.4) : "transparent"
                    onClicked: currentTab = 0
                }

                Button {
                    text: qsTr("Import")
                    preferredWidth: (parent.width - Theme.paddingSmall) / 2
                    color: currentTab === 1 ? Theme.highlightColor : Theme.primaryColor
                    backgroundColor: currentTab === 1 ? Theme.rgba(Theme.highlightBackgroundColor, 0.4) : "transparent"
                    onClicked: currentTab = 1
                }
            }

            // ==================== TAB 0: CHAT & TEMPLATES ====================
            Column {
                id: chatTabCol
                width: parent.width
                spacing: Theme.paddingMedium
                visible: currentTab === 0

                // Quick Preset Action Templates
                AiTemplateBar {
                    enabled: !agentBridge.agent_busy && assistantPage.hasContextOrInput
                    onTemplateSelected: function(tpl) {
                        assistantPage.applyConfig()
                        agentBridge.run_template(tpl, promptBar.text, contextFilename, contextContent)
                        promptBar.text = ""
                        assistantPage.scrollToBottom()
                    }
                }

                // Active Processing Banner (immediate feedback)
                Rectangle {
                    width: parent.width - Theme.horizontalPageMargin * 2
                    anchors.horizontalCenter: parent.horizontalCenter
                    height: Theme.itemSizeExtraSmall
                    radius: Theme.paddingSmall
                    color: Theme.rgba(Theme.highlightBackgroundColor, 0.25)
                    border.color: Theme.rgba(Theme.highlightColor, 0.4)
                    border.width: 1
                    visible: agentBridge.agent_busy && currentTab === 0

                    Row {
                        anchors.centerIn: parent
                        spacing: Theme.paddingMedium

                        BusyIndicator {
                            size: BusyIndicatorSize.ExtraSmall
                            running: agentBridge.agent_busy
                            anchors.verticalCenter: parent.verticalCenter
                        }

                        Label {
                            text: agentBridge.streaming_text.length > 0 ? qsTr("AI is generating response...") : qsTr("AI is analyzing & processing...")
                            font.pixelSize: Theme.fontSizeSmall
                            color: Theme.highlightColor
                            anchors.verticalCenter: parent.verticalCenter
                        }
                    }
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

                // Pending Confirmation Card
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

                // Chat Messages & Streaming View
                AiConversationView {
                    messagesJson: agentBridge.messages_json
                    agentBusy: agentBridge.agent_busy
                    streamingText: currentTab === 0 ? agentBridge.streaming_text : ""
                }

                // Input Bar with Mic & Live Waveform
                AiPromptBar {
                    id: promptBar
                    agentBusy: agentBridge.agent_busy
                    isSpeechRecording: assistantPage.isSpeechRecording
                    isSpeechTranscribing: assistantPage.isSpeechTranscribing
                    liveAudioLevel: assistantPage.liveAudioLevel
                    liveWaveform: assistantPage.liveWaveform
                    sttEnabled: (typeof app !== "undefined" && app.sttEnabled !== undefined) ? app.sttEnabled : true
                    onSubmitPrompt: function(txt) {
                        assistantPage.applyConfig()
                        agentBridge.send_prompt(txt)
                        promptBar.text = ""
                        assistantPage.scrollToBottom()
                    }
                    onToggleMic: assistantPage.handleMicClick()
                    onCancelRecording: assistantPage.cancelActiveRecording()
                }
            }

            // ==================== TAB 1: IMPORT ASSISTANT ====================
            AiImportTab {
                id: importTab
                visible: currentTab === 1
                agentBusy: agentBridge.agent_busy
                isFetching: agentBridge.is_fetching
                streamingText: currentTab === 1 ? agentBridge.streaming_text : ""
                lastCreatedNote: agentBridge.last_created_note
                messagesJson: agentBridge.messages_json
                isSpeechRecording: assistantPage.isSpeechRecording
                isSpeechTranscribing: assistantPage.isSpeechTranscribing
                sttEnabled: (typeof app !== "undefined" && app.sttEnabled !== undefined) ? app.sttEnabled : true
                onConvertRequested: function(src, title, mode, prompt) {
                    assistantPage.applyConfig()
                    agentBridge.import_text(src, title, mode, prompt)
                }
                onFetchUrlRequested: function(url) {
                    agentBridge.fetch_url_content(url)
                }
                onReadFileRequested: function(path) {
                    agentBridge.read_local_file(path)
                }
                onToggleMic: assistantPage.handleMicClick()
                onOpenCreatedNote: function(noteTitle) {
                    pageStack.push(Qt.resolvedUrl("PageView.qml"), {
                        pageName: noteTitle
                    })
                    bridge.load_page(noteTitle)
                }
                onClipboardPasted: {
                    if (Clipboard.text && Clipboard.text.length > 0) {
                        importTab.sourceText = Clipboard.text
                    } else {
                        remorsePopup.execute(qsTr("Clipboard is empty"), function() {})
                    }
                }
            }
        }
    }
}
