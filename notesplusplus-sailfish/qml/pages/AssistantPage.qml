import QtQuick 2.6
import Sailfish.Silica 1.0
import harbour.notesplus 1.0
import "../components"

Page {
    id: assistantPage
    allowedOrientations: Orientation.All

    property var attachedNotes: []
    property string contextFilename: ""
    property string contextContent: ""
    property string extraContext: ""

    readonly property bool hasNote: (attachedNotes && attachedNotes.length > 0) || (contextFilename.length > 0) || (contextContent.length > 0)
    readonly property bool hasContext: hasNote || (extraContext.length > 0)
    property bool hasContextOrInput: hasContext || (promptBar && promptBar.text && promptBar.text.trim().length > 0)

    function getCombinedNotesContent() {
        if (!attachedNotes || attachedNotes.length === 0) {
            return contextContent || ""
        }
        if (attachedNotes.length === 1) {
            return attachedNotes[0].content || ""
        }
        var parts = []
        for (var i = 0; i < attachedNotes.length; i++) {
            var n = attachedNotes[i]
            var titleOrFn = (n.title && n.title.length > 0) ? n.title : n.filename
            parts.push("=== Note: " + titleOrFn + " (" + n.filename + ") ===\n" + (n.content || ""))
        }
        return parts.join("\n\n")
    }

    function getCombinedFilenames() {
        if (!attachedNotes || attachedNotes.length === 0) {
            return contextFilename || ""
        }
        var names = []
        for (var i = 0; i < attachedNotes.length; i++) {
            names.push(attachedNotes[i].filename)
        }
        return names.join(", ")
    }

    function syncSessionContext(resetAgentSession) {
        if (resetAgentSession === undefined) resetAgentSession = true
        contextFilename = getCombinedFilenames()
        contextContent = getCombinedNotesContent()
        if (resetAgentSession && typeof agentBridge !== "undefined" && agentBridge) {
            agentBridge.reset_session(contextFilename, contextContent, extraContext)
        }
    }

    function attachNote(filename, content, title) {
        if (!filename && !content) return
        var found = false
        var updated = []
        var curNotes = attachedNotes ? attachedNotes.slice(0) : []
        for (var i = 0; i < curNotes.length; i++) {
            if (curNotes[i].filename === filename) {
                updated.push({
                    filename: filename,
                    title: title || curNotes[i].title || filename,
                    content: content
                })
                found = true
            } else {
                updated.push(curNotes[i])
            }
        }
        if (!found) {
            updated.push({
                filename: filename,
                title: title || filename,
                content: content
            })
        }
        attachedNotes = updated
        syncSessionContext(true)
        remorsePopup.execute(qsTr("Attached note: %1").arg(title || filename), function() {})
    }

    function setAttachedNotes(notesList) {
        attachedNotes = notesList || []
        syncSessionContext(true)
        if (attachedNotes.length === 1) {
            remorsePopup.execute(qsTr("Attached note: %1").arg(attachedNotes[0].title || attachedNotes[0].filename), function() {})
        } else if (attachedNotes.length > 1) {
            remorsePopup.execute(qsTr("Attached %1 notes").arg(attachedNotes.length), function() {})
        } else {
            remorsePopup.execute(qsTr("Detached note context"), function() {})
        }
    }

    function detachNote(filename, index) {
        var updated = []
        var detachedName = filename || ""
        var curNotes = attachedNotes ? attachedNotes.slice(0) : []
        for (var i = 0; i < curNotes.length; i++) {
            if (index !== undefined && index !== null && index >= 0) {
                if (i === index) {
                    detachedName = curNotes[i].title || curNotes[i].filename
                    continue
                }
            } else if (filename && curNotes[i].filename === filename) {
                detachedName = curNotes[i].title || curNotes[i].filename
                continue
            }
            updated.push(curNotes[i])
        }
        attachedNotes = updated
        syncSessionContext(true)
        if (detachedName && detachedName.length > 0) {
            remorsePopup.execute(qsTr("Detached note: %1").arg(detachedName), function() {})
        } else {
            remorsePopup.execute(qsTr("Detached note context"), function() {})
        }
    }

    function attachClipboard() {
        if (Clipboard.text && Clipboard.text.length > 0) {
            extraContext = Clipboard.text
            syncSessionContext(true)
            remorsePopup.execute(qsTr("Attached clipboard context"), function() {})
        } else {
            remorsePopup.execute(qsTr("Clipboard is empty"), function() {})
        }
    }

    function detachExtraContext() {
        extraContext = ""
        syncSessionContext(true)
        remorsePopup.execute(qsTr("Detached clipboard context"), function() {})
    }

    function clearAllContext() {
        attachedNotes = []
        contextFilename = ""
        contextContent = ""
        extraContext = ""
        if (typeof agentBridge !== "undefined" && agentBridge) {
            agentBridge.reset_session("", "", "")
        }
        remorsePopup.execute(qsTr("Cleared all context"), function() {})
    }

    function openAttachNoteDialog() {
        var preselected = []
        var curNotes = attachedNotes || []
        for (var i = 0; i < curNotes.length; i++) {
            preselected.push(curNotes[i].filename)
        }
        var dialog = pageStack.push(Qt.resolvedUrl("PageLinkDialog.qml"), {
            mode: "select",
            allowMultiple: true,
            selectedFilenames: preselected
        })
        dialog.accepted.connect(function() {
            if (dialog.selectedPages && dialog.selectedPages.length > 0) {
                var notesToAdd = []
                for (var i = 0; i < dialog.selectedPages.length; i++) {
                    var fn = dialog.selectedPages[i].filename
                    if (fn && fn.length > 0) {
                        var content = bridge.get_page_source(fn)
                        notesToAdd.push({
                            filename: fn,
                            title: dialog.selectedPages[i].title || fn.replace(/\.adoc$/, ""),
                            content: content
                        })
                    }
                }
                assistantPage.setAttachedNotes(notesToAdd)
            } else if (dialog.targetPageFilename && dialog.targetPageFilename.length > 0) {
                var fn2 = dialog.targetPageFilename
                var content2 = bridge.get_page_source(fn2)
                assistantPage.attachNote(fn2, content2, dialog.targetPageTitle || fn2)
            }
        })
    }

    Component.onCompleted: {
        if ((!attachedNotes || attachedNotes.length === 0) && (contextFilename.length > 0 || contextContent.length > 0)) {
            attachedNotes = [{
                filename: contextFilename,
                title: contextFilename.replace(/\.adoc$/, ""),
                content: contextContent
            }]
            syncSessionContext(false)
        }
    }

    function appendTranscribedText(text) {
        if (!text || text.trim().length === 0) return
        var clean = text.trim()
        if (promptBar.text.length > 0) {
            promptBar.text = promptBar.text + " " + clean
        } else {
            promptBar.text = clean
        }
        assistantPage.scrollToBottom()
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

    Connections {
        target: (typeof speechBridge !== "undefined" && speechBridge) ? speechBridge : null

        onTranscription_completed: function(text) {
            if (assistantPage.status !== PageStatus.Active) return
            assistantPage.appendTranscribedText(text)
        }

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
                    var noteCount = assistantPage.attachedNotes ? assistantPage.attachedNotes.length : 0
                    var hasClip = assistantPage.extraContext.length > 0
                    if (noteCount > 1) {
                        return hasClip ? qsTr("Context: %1 notes + Clipboard").arg(noteCount)
                                       : qsTr("Context: %1 notes").arg(noteCount)
                    } else if (noteCount === 1) {
                        var fn = assistantPage.attachedNotes[0].title || assistantPage.attachedNotes[0].filename
                        return hasClip ? qsTr("Context: %1 + Clipboard").arg(fn)
                                       : qsTr("Context: %1").arg(fn)
                    } else if (assistantPage.contextFilename.length > 0) {
                        return hasClip ? qsTr("Context: %1 + Clipboard").arg(assistantPage.contextFilename)
                                       : qsTr("Context: %1").arg(assistantPage.contextFilename)
                    } else if (hasClip) {
                        return qsTr("Context: Clipboard text")
                    } else {
                        return qsTr("No context attached")
                    }
                }
            }

            // Chat Messages & Streaming View
            AiConversationView {
                messagesJson: agentBridge.messages_json
                agentBusy: agentBridge.agent_busy
                streamingText: agentBridge.streaming_text
                stepStatus: agentBridge.step_status
                stepDetail: agentBridge.step_detail
                onResendRequested: function(txt) {
                    if (txt && txt.trim().length > 0 && !agentBridge.agent_busy) {
                        assistantPage.applyConfig()
                        agentBridge.send_prompt(txt)
                        assistantPage.scrollToBottom()
                    }
                }
                onCancelRequested: agentBridge.cancel_operation()
            }

            // Undo Banner
            UndoBanner {
                width: parent.width - Theme.horizontalPageMargin * 2
                anchors.horizontalCenter: parent.horizontalCenter
                visible: agentBridge.can_undo
                onUndoTriggered: {
                    agentBridge.undo_last_action()
                }
            }

            // Pending Confirmation Card
            ConfirmationCard {
                width: parent.width - Theme.horizontalPageMargin * 2
                anchors.horizontalCenter: parent.horizontalCenter
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

            // Integrated Prompt Bar & Context Actions
            AiPromptBar {
                id: promptBar
                agentBusy: agentBridge.agent_busy
                attachedNotes: assistantPage.attachedNotes
                contextFilename: assistantPage.contextFilename
                contextContent: assistantPage.contextContent
                extraContext: assistantPage.extraContext
                isSpeechRecording: assistantPage.isSpeechRecording
                isSpeechTranscribing: assistantPage.isSpeechTranscribing
                liveAudioLevel: assistantPage.liveAudioLevel
                liveWaveform: assistantPage.liveWaveform
                sttEnabled: (typeof app !== "undefined" && app.sttEnabled !== undefined) ? app.sttEnabled : true
                onShowInstructionsChanged: {
                    assistantPage.scrollToBottom()
                }
                onSubmitPrompt: function(txt) {
                    assistantPage.applyConfig()
                    agentBridge.send_prompt(txt)
                    promptBar.text = ""
                    assistantPage.scrollToBottom()
                }
                onToggleMic: assistantPage.handleMicClick()
                onCancelRecording: assistantPage.cancelActiveRecording()
                onAttachNoteRequested: assistantPage.openAttachNoteDialog()
                onDetachNoteRequested: function(filename, index) {
                    assistantPage.detachNote(filename, index)
                }
                onAttachClipboardRequested: assistantPage.attachClipboard()
                onDetachExtraContextRequested: assistantPage.detachExtraContext()
                onClearAllContextRequested: assistantPage.clearAllContext()
                onInstructionSelected: function(item) {
                    var tmpl = item.instruction || ""
                    if (!tmpl && typeof app !== "undefined" && app.getDefaultAiInstruction) {
                        var def = app.getDefaultAiInstruction(item.id)
                        if (def && def.instruction) {
                            tmpl = def.instruction
                        }
                    }
                    var fn = assistantPage.getCombinedFilenames()
                    var ctx = assistantPage.getCombinedNotesContent()
                    var extra = assistantPage.extraContext
                    var inp = promptBar.text.trim()

                    var resolvedPrompt = tmpl
                    if (resolvedPrompt.indexOf("{filename}") !== -1) {
                        resolvedPrompt = resolvedPrompt.replace(/\{filename\}/g, fn)
                    }
                    if (resolvedPrompt.indexOf("{content}") !== -1) {
                        resolvedPrompt = resolvedPrompt.replace(/\{content\}/g, ctx)
                    }
                    if (resolvedPrompt.indexOf("{input}") !== -1) {
                        resolvedPrompt = resolvedPrompt.replace(/\{input\}/g, inp)
                    }
                    if (resolvedPrompt.indexOf("{context}") !== -1) {
                        var combinedContext = ctx
                        if (extra.length > 0) {
                            combinedContext = combinedContext.length > 0 ? (combinedContext + "\n\n" + extra) : extra
                        }
                        resolvedPrompt = resolvedPrompt.replace(/\{context\}/g, combinedContext)
                    }

                    // Populate prompt field directly for transparent inspection and tweaking
                    promptBar.text = resolvedPrompt
                    promptBar.showInstructions = false
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
        }
    }
}
