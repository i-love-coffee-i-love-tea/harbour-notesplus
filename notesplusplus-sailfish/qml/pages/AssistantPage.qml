import QtQuick 2.0
import Sailfish.Silica 1.0
import harbour.notesplus 1.0
import "../components"

Page {
    id: assistantPage
    allowedOrientations: Orientation.All

    property string contextFilename: ""
    property string contextContent: ""
    property string extraContext: ""

    readonly property bool hasContext: (contextFilename.length > 0) || (contextContent.length > 0) || (extraContext.length > 0)
    property bool hasContextOrInput: hasContext || (promptBar && promptBar.text && promptBar.text.trim().length > 0)

    function attachNote(filename, content) {
        contextFilename = filename
        contextContent = content
        agentBridge.reset_session(contextFilename, contextContent, extraContext)
        remorsePopup.execute(qsTr("Attached note: %1").arg(filename), function() {})
    }

    function detachNote() {
        contextFilename = ""
        contextContent = ""
        agentBridge.reset_session("", "", extraContext)
        remorsePopup.execute(qsTr("Detached note context"), function() {})
    }

    function attachClipboard() {
        if (Clipboard.text && Clipboard.text.length > 0) {
            extraContext = Clipboard.text
            agentBridge.reset_session(contextFilename, contextContent, extraContext)
            remorsePopup.execute(qsTr("Attached clipboard context"), function() {})
        } else {
            remorsePopup.execute(qsTr("Clipboard is empty"), function() {})
        }
    }

    function detachExtraContext() {
        extraContext = ""
        agentBridge.reset_session(contextFilename, contextContent, "")
        remorsePopup.execute(qsTr("Detached clipboard context"), function() {})
    }

    function clearAllContext() {
        contextFilename = ""
        contextContent = ""
        extraContext = ""
        agentBridge.reset_session("", "", "")
        remorsePopup.execute(qsTr("Cleared all context"), function() {})
    }

    function openAttachNoteDialog() {
        var dialog = pageStack.push(Qt.resolvedUrl("PageLinkDialog.qml"), {
            mode: "select"
        })
        dialog.accepted.connect(function() {
            var fn = dialog.targetPageFilename
            if (fn && fn.length > 0) {
                var content = bridge.get_page_source(fn)
                assistantPage.attachNote(fn, content)
            }
        })
    }

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
            if (promptBar.text.length > 0) {
                promptBar.text = promptBar.text + " " + clean
            } else {
                promptBar.text = clean
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
            if (contextFilename.length > 0 || extraContext.length > 0) {
                agentBridge.reset_session(contextFilename, contextContent, extraContext)
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
                onClicked: pageStack.push(Qt.resolvedUrl("ModelDownloadDialog.qml"))
            }

            MenuItem {
                text: qsTr("Manage AI Instructions...")
                onClicked: pageStack.push(Qt.resolvedUrl("CustomInstructionsPage.qml"))
            }

            MenuItem {
                text: qsTr("Detach All Context")
                visible: assistantPage.hasContext
                onClicked: assistantPage.clearAllContext()
            }

            MenuItem {
                text: qsTr("Clear Conversation")
                onClicked: {
                    agentBridge.reset_session(contextFilename, contextContent, extraContext)
                }
            }
        }

        Column {
            id: mainColumn
            width: parent.width
            spacing: Theme.paddingMedium

            PageHeader {
                title: qsTr("AI Assistant")
                description: {
                    if (assistantPage.hasContext) {
                        if (contextFilename.length > 0 && extraContext.length > 0) {
                            return qsTr("Context: %1 + Clipboard").arg(contextFilename)
                        } else if (contextFilename.length > 0) {
                            return qsTr("Context: %1").arg(contextFilename)
                        } else {
                            return qsTr("Context: Clipboard text")
                        }
                    } else {
                        return qsTr("No context attached")
                    }
                }
            }

            // Context Display Card (shows attached note/clipboard or explicit "No context" state with attach actions)
            AiContextCard {
                contextFilename: assistantPage.contextFilename
                contextContent: assistantPage.contextContent
                extraContext: assistantPage.extraContext
                agentBusy: agentBridge.agent_busy
                onAttachNoteRequested: assistantPage.openAttachNoteDialog()
                onDetachNoteRequested: assistantPage.detachNote()
                onAttachClipboardRequested: assistantPage.attachClipboard()
                onDetachExtraContextRequested: assistantPage.detachExtraContext()
                onClearAllContextRequested: assistantPage.clearAllContext()
            }

            // Chat Messages & Streaming View
            AiConversationView {
                messagesJson: agentBridge.messages_json
                agentBusy: agentBridge.agent_busy
                streamingText: agentBridge.streaming_text
            }

            // Active Processing Banner (immediate feedback)
            Rectangle {
                width: parent.width - Theme.horizontalPageMargin * 2
                anchors.horizontalCenter: parent.horizontalCenter
                height: Theme.itemSizeExtraSmall
                radius: Theme.paddingSmall
                color: Theme.rgba(Theme.highlightBackgroundColor, 0.25)
                border.color: Theme.rgba(Theme.primaryColor, 0.3)
                border.width: 1
                visible: agentBridge.agent_busy

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
                        color: Theme.primaryColor
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

            // Quick Preset & Custom Action Instructions
            AiTemplateBar {
                id: templateBar
                enabled: !agentBridge.agent_busy
                agentBusy: agentBridge.agent_busy
                hasContextOrInput: assistantPage.hasContextOrInput
                onExpandedChanged: {
                    assistantPage.scrollToBottom()
                }
                onInstructionSelected: function(item) {
                    if (!assistantPage.hasContextOrInput) {
                        assistantPage.openAttachNoteDialog()
                        return
                    }
                    assistantPage.applyConfig()
                    var ctx = contextContent
                    if (extraContext.length > 0) {
                        ctx = ctx.length > 0 ? (ctx + "\n\n" + extraContext) : extraContext
                    }
                    if (item.instruction && item.instruction.length > 0) {
                        agentBridge.run_custom_instruction(item.instruction, promptBar.text, contextFilename, ctx)
                    } else if (item.id) {
                        agentBridge.run_template(item.id, promptBar.text, contextFilename, ctx)
                    }
                    promptBar.text = ""
                    assistantPage.scrollToBottom()
                }
                onEditInstructionRequested: function(item) {
                    var inst = item.instruction || ""
                    if (!inst && typeof app !== "undefined" && app.getDefaultAiInstruction) {
                        var def = app.getDefaultAiInstruction(item.id)
                        if (def && def.instruction) {
                            inst = def.instruction
                        }
                    }
                    pageStack.push(Qt.resolvedUrl("CustomInstructionDialog.qml"), {
                        "instructionId": item.id || "",
                        "initialButtonText": item.buttonText || item.title || "",
                        "initialIcon": item.icon || "icon-m-note",
                        "initialInstruction": inst,
                        "isEdit": true
                    })
                }
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
    }
}
