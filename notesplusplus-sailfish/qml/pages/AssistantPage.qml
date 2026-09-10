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

    property bool hasContextOrInput: (contextContent.length > 0) || (promptField.text && promptField.text.trim().length > 0)
    property bool showUrlInput: false
    property bool showFileInput: false
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
                if (promptField.text.length > 0) {
                    promptField.text = promptField.text + " " + clean
                } else {
                    promptField.text = clean
                }
            } else {
                if (sourceTextArea.text.length > 0) {
                    sourceTextArea.text = sourceTextArea.text + " " + clean
                } else {
                    sourceTextArea.text = clean
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
            // Ensure recording UI state is cleared on backend errors.
            if (typeof speechBridge !== "undefined" && speechBridge && speechBridge.is_recording) {
                speechBridge.cancel_recording()
            }
        }
    }

    Connections {
        target: app
        onAiProviderChanged: assistantPage.applyConfig()
        onAiEndpointChanged: assistantPage.applyConfig()
        onAiModelChanged: assistantPage.applyConfig()
        onAiApiKeyChanged: assistantPage.applyConfig()
        onAiTimeoutChanged: assistantPage.applyConfig()
        onAiAutoAllowReadChanged: assistantPage.applyConfig()
        onAiAutoAllowCreateChanged: assistantPage.applyConfig()
        onAiRequireConfirmEditChanged: assistantPage.applyConfig()
        onAiAllowSelfSignedChanged: assistantPage.applyConfig()
    }

    AgentBridge {
        id: agentBridge

        Component.onCompleted: {
            assistantPage.applyConfig()

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
            sourceTextArea.text = result
            assistantPage.showUrlInput = false
            assistantPage.showFileInput = false
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
                        promptField.text = qsTr("Please analyze the clipboard content.")
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
                        sourceTextArea.text = Clipboard.text
                    } else {
                        remorsePopup.execute(qsTr("Clipboard is empty"), function() {})
                    }
                }
            }
            MenuItem {
                text: qsTr("Clear Import Fields")
                visible: currentTab === 1
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
                SilicaFlickable {
                    width: parent.width
                    height: templateRow.height + Theme.paddingSmall
                    contentWidth: templateRow.width + Theme.horizontalPageMargin * 2
                    clip: true

                    Row {
                        id: templateRow
                        x: Theme.horizontalPageMargin
                        spacing: Theme.paddingSmall

                        Button {
                            text: qsTr("✨ Beautify")
                            preferredWidth: Theme.buttonWidthExtraSmall
                            enabled: !agentBridge.agent_busy && assistantPage.hasContextOrInput
                            opacity: enabled ? 1.0 : 0.4
                            onClicked: {
                                assistantPage.applyConfig()
                                agentBridge.run_template("beautify", promptField.text, contextFilename, contextContent)
                                promptField.text = ""
                                assistantPage.scrollToBottom()
                            }
                        }

                        Button {
                            text: qsTr("📋 Extract To-Dos")
                            preferredWidth: Theme.buttonWidthExtraSmall
                            enabled: !agentBridge.agent_busy && assistantPage.hasContextOrInput
                            opacity: enabled ? 1.0 : 0.4
                            onClicked: {
                                assistantPage.applyConfig()
                                agentBridge.run_template("extract_todos", promptField.text, contextFilename, contextContent)
                                promptField.text = ""
                                assistantPage.scrollToBottom()
                            }
                        }

                        Button {
                            text: qsTr("✍️ Fix Grammar")
                            preferredWidth: Theme.buttonWidthExtraSmall
                            enabled: !agentBridge.agent_busy && assistantPage.hasContextOrInput
                            opacity: enabled ? 1.0 : 0.4
                            onClicked: {
                                assistantPage.applyConfig()
                                agentBridge.run_template("fix_grammar", promptField.text, contextFilename, contextContent)
                                promptField.text = ""
                                assistantPage.scrollToBottom()
                            }
                        }

                        Button {
                            text: qsTr("📝 Expand & Draft")
                            preferredWidth: Theme.buttonWidthExtraSmall
                            enabled: !agentBridge.agent_busy && assistantPage.hasContextOrInput
                            opacity: enabled ? 1.0 : 0.4
                            onClicked: {
                                assistantPage.applyConfig()
                                agentBridge.run_template("expand_draft", promptField.text, contextFilename, contextContent)
                                promptField.text = ""
                                assistantPage.scrollToBottom()
                            }
                        }

                        Button {
                            text: qsTr("🌐 External Text")
                            preferredWidth: Theme.buttonWidthExtraSmall
                            enabled: !agentBridge.agent_busy && assistantPage.hasContextOrInput
                            opacity: enabled ? 1.0 : 0.4
                            onClicked: {
                                assistantPage.applyConfig()
                                agentBridge.run_template("analyze_external", promptField.text, contextFilename, contextContent)
                                promptField.text = ""
                                assistantPage.scrollToBottom()
                            }
                        }
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

                // Chat Messages Repeater
                Column {
                    width: parent.width - Theme.horizontalPageMargin * 2
                    anchors.horizontalCenter: parent.horizontalCenter
                    spacing: Theme.paddingMedium

                    Repeater {
                        model: {
                            try {
                                var list = JSON.parse(agentBridge.messages_json)
                                // Filter out internal system prompt
                                return list.filter(function(m) { return m.role !== "system" })
                            } catch (e) {
                                return []
                            }
                        }

                        delegate: Item {
                            width: parent.width
                            height: msgBubble.height + Theme.paddingSmall

                            Rectangle {
                                id: msgBubble
                                width: parent.width
                                height: msgTextCol.height + Theme.paddingMedium * 2
                                radius: Theme.paddingSmall
                                color: {
                                    if (modelData.role === "user") {
                                        return Theme.rgba(Theme.highlightBackgroundColor, 0.4)
                                    } else if (modelData.role === "tool") {
                                        return Theme.rgba(Theme.primaryColor, 0.08)
                                    } else {
                                        return Theme.rgba(Theme.highlightBackgroundColor, 0.18)
                                    }
                                }

                                Column {
                                    id: msgTextCol
                                    anchors.left: parent.left
                                    anchors.right: parent.right
                                    anchors.top: parent.top
                                    anchors.margins: Theme.paddingMedium
                                    spacing: Theme.paddingSmall

                                    Row {
                                        spacing: Theme.paddingSmall
                                        Label {
                                            text: {
                                                if (modelData.role === "user") return qsTr("You")
                                                if (modelData.role === "tool") return qsTr("🔧 Tool Output")
                                                return qsTr("🤖 Assistant")
                                            }
                                            font.pixelSize: Theme.fontSizeExtraSmall
                                            font.bold: true
                                            color: Theme.highlightColor
                                        }
                                    }

                                    Label {
                                        width: parent.width
                                        text: modelData.content || (modelData.tool_calls ? qsTr("Running note tools...") : "")
                                        font.pixelSize: Theme.fontSizeSmall
                                        color: Theme.primaryColor
                                        wrapMode: Text.Wrap
                                    }
                                }
                            }
                        }
                    }

                    // Live Streaming Assistant Bubble
                    Item {
                        width: parent.width
                        height: liveMsgBubble.height + Theme.paddingSmall
                        visible: agentBridge.agent_busy && agentBridge.streaming_text.length > 0 && currentTab === 0

                        Rectangle {
                            id: liveMsgBubble
                            width: parent.width
                            height: liveMsgTextCol.height + Theme.paddingMedium * 2
                            radius: Theme.paddingSmall
                            color: Theme.rgba(Theme.highlightBackgroundColor, 0.18)

                            Column {
                                id: liveMsgTextCol
                                anchors.left: parent.left
                                anchors.right: parent.right
                                anchors.top: parent.top
                                anchors.margins: Theme.paddingMedium
                                spacing: Theme.paddingSmall

                                Row {
                                    spacing: Theme.paddingSmall
                                    Label {
                                        text: qsTr("🤖 Assistant")
                                        font.pixelSize: Theme.fontSizeExtraSmall
                                        font.bold: true
                                        color: Theme.highlightColor
                                    }
                                    BusyIndicator {
                                        size: BusyIndicatorSize.ExtraSmall
                                        running: true
                                        anchors.verticalCenter: parent.verticalCenter
                                    }
                                }

                                Label {
                                    width: parent.width
                                    text: agentBridge.streaming_text
                                    font.pixelSize: Theme.fontSizeSmall
                                    color: Theme.primaryColor
                                    wrapMode: Text.Wrap
                                }
                            }
                        }
                    }
                }

                // Busy Indicator (when waiting for first token or executing tools)
                Item {
                    width: parent.width
                    height: Theme.itemSizeMedium
                    visible: agentBridge.agent_busy && agentBridge.streaming_text.length === 0 && currentTab === 0

                    BusyIndicator {
                        anchors.centerIn: parent
                        running: agentBridge.agent_busy && agentBridge.streaming_text.length === 0 && currentTab === 0
                        size: BusyIndicatorSize.Small
                    }
                }

                // Recording / Transcribing Status Indicator
                Rectangle {
                    width: parent.width - Theme.horizontalPageMargin * 2
                    anchors.horizontalCenter: parent.horizontalCenter
                    height: (assistantPage.isSpeechRecording || assistantPage.isSpeechTranscribing) ? Theme.itemSizeExtraSmall : 0
                    color: Theme.rgba(Theme.highlightBackgroundColor, 0.15)
                    radius: Theme.paddingSmall
                    border.color: (assistantPage.isSpeechRecording && assistantPage.liveAudioLevel > 0.06) ?
                                  Theme.rgba(Theme.highlightColor, 0.6) : Theme.rgba(Theme.highlightColor, 0.3)
                    border.width: 1
                    visible: assistantPage.isSpeechRecording || assistantPage.isSpeechTranscribing
                    clip: true

                    Behavior on height { NumberAnimation { duration: 150 } }
                    Behavior on border.color { ColorAnimation { duration: 100 } }

                    Row {
                        anchors.centerIn: parent
                        spacing: Theme.paddingMedium

                        BusyIndicator {
                            size: BusyIndicatorSize.ExtraSmall
                            running: assistantPage.isSpeechTranscribing
                            visible: assistantPage.isSpeechTranscribing
                            anchors.verticalCenter: parent.verticalCenter
                        }

                        // Live Multi-Bar Waveform Visualizer
                        Row {
                            spacing: 3
                            anchors.verticalCenter: parent.verticalCenter
                            visible: assistantPage.isSpeechRecording

                            Repeater {
                                model: 7
                                Rectangle {
                                    id: waveBar
                                    width: 3
                                    readonly property real barVal: (assistantPage.liveWaveform && assistantPage.liveWaveform.length > index) ? assistantPage.liveWaveform[index] : 0.0
                                    height: Math.max(4, Math.min(Theme.itemSizeExtraSmall - Theme.paddingMedium, Math.round(barVal * (Theme.itemSizeExtraSmall - Theme.paddingMedium))))
                                    radius: 1.5
                                    color: (barVal > 0.06) ? Theme.highlightColor : Theme.secondaryColor
                                    anchors.verticalCenter: parent.verticalCenter

                                    Behavior on height { NumberAnimation { duration: 50 } }
                                    Behavior on color { ColorAnimation { duration: 80 } }
                                }
                            }
                        }

                        Label {
                            text: {
                                if (assistantPage.isSpeechTranscribing) {
                                    return qsTr("Transcribing speech with offline Whisper...")
                                }
                                if (assistantPage.liveAudioLevel > 0.06) {
                                    var percent = Math.round(assistantPage.liveAudioLevel * 100)
                                    return qsTr("Hearing voice (%1%)... Tap mic to finish").arg(percent)
                                }
                                return qsTr("Listening... Speak into microphone")
                            }
                            color: (assistantPage.isSpeechRecording && assistantPage.liveAudioLevel > 0.06) ?
                                   Theme.primaryColor : Theme.highlightColor
                            font.pixelSize: Theme.fontSizeSmall
                            anchors.verticalCenter: parent.verticalCenter
                        }

                        Label {
                            text: qsTr("Cancel")
                            color: Theme.secondaryHighlightColor
                            font.pixelSize: Theme.fontSizeExtraSmall
                            visible: assistantPage.isSpeechRecording
                            anchors.verticalCenter: parent.verticalCenter
                            MouseArea {
                                anchors.fill: parent
                                anchors.margins: -Theme.paddingSmall
                                onClicked: assistantPage.cancelActiveRecording()
                            }
                        }
                    }
                }

                // Input Area
                Column {
                    width: parent.width - Theme.horizontalPageMargin * 2
                    anchors.horizontalCenter: parent.horizontalCenter
                    spacing: Theme.paddingSmall

                    TextArea {
                        id: promptField
                        width: parent.width
                        height: Math.max(Theme.itemSizeLarge, implicitHeight)
                        placeholderText: contextFilename.length > 0 ? qsTr("Ask assistant or run template on note...") : qsTr("Ask assistant or enter text for templates...")
                        label: qsTr("Prompt")
                        enabled: !agentBridge.agent_busy && !assistantPage.isSpeechTranscribing
                    }

                    Row {
                        width: parent.width
                        layoutDirection: Qt.RightToLeft
                        spacing: Theme.paddingSmall

                        IconButton {
                            id: sendBtn
                            icon.source: "image://theme/icon-m-send"
                            enabled: !agentBridge.agent_busy && !assistantPage.isSpeechTranscribing && promptField.text.length > 0
                            onClicked: {
                                if (promptField.text.length > 0) {
                                    assistantPage.applyConfig()
                                    agentBridge.send_prompt(promptField.text)
                                    promptField.text = ""
                                }
                            }
                        }

                        Item {
                            id: micBtnContainer
                            width: micBtn.width
                            height: micBtn.height
                            visible: (typeof app !== "undefined" && app.sttEnabled !== undefined) ? app.sttEnabled : true

                        // Dynamic audio volume halo reacting directly to live microphone voice input
                        Rectangle {
                            id: micPulseHalo
                            anchors.centerIn: parent
                            width: parent.width + Theme.paddingSmall + Math.round(assistantPage.liveAudioLevel * 28)
                            height: parent.height + Theme.paddingSmall + Math.round(assistantPage.liveAudioLevel * 28)
                            radius: width / 2
                            color: (assistantPage.liveAudioLevel > 0.06) ? "#44ff88" : "#ff4444"
                            opacity: assistantPage.isSpeechRecording ? Math.min(0.85, 0.25 + assistantPage.liveAudioLevel * 0.6) : 0.0
                            visible: assistantPage.isSpeechRecording

                            Behavior on width { NumberAnimation { duration: 60 } }
                            Behavior on height { NumberAnimation { duration: 60 } }
                            Behavior on opacity { NumberAnimation { duration: 60 } }
                            Behavior on color { ColorAnimation { duration: 100 } }
                        }

                        IconButton {
                            id: micBtn
                            anchors.centerIn: parent
                            icon.source: assistantPage.isSpeechRecording ? "image://theme/icon-m-clear" : "image://theme/icon-m-mic"
                            highlighted: assistantPage.isSpeechRecording
                            enabled: !agentBridge.agent_busy && !assistantPage.isSpeechTranscribing
                            onClicked: {
                                if (typeof speechBridge === "undefined" || !speechBridge) {
                                    remorsePopup.execute(qsTr("Speech recognition is unavailable."), function() {})
                                    return
                                }
                                if (speechBridge.is_recording) {
                                    // Stop capture and queue offline Whisper transcription.
                                    speechBridge.stop_recording_and_transcribe()
                                } else {
                                    if (!speechBridge.has_installed_models) {
                                        remorsePopup.execute(qsTr("No speech model installed. Please download a model."), function() {})
                                        pageStack.push(Qt.resolvedUrl("ModelDownloadDialog.qml"))
                                        return
                                    }
                                    if (!speechBridge.start_recording()) {
                                        // error_occurred signal already surfaces the reason
                                        return
                                    }
                                }
                            }
                        }
                    }
                    }
                }
            }

            // ==================== TAB 1: IMPORT ASSISTANT ====================
            Column {
                id: importTabCol
                width: parent.width
                spacing: Theme.paddingMedium
                visible: currentTab === 1

                SectionHeader {
                    text: qsTr("Source Content")
                }

                Row {
                    anchors.horizontalCenter: parent.horizontalCenter
                    spacing: Theme.paddingSmall

                    Button {
                        text: qsTr("📋 Paste Clipboard")
                        preferredWidth: Theme.buttonWidthExtraSmall
                        enabled: !agentBridge.agent_busy
                        onClicked: {
                            if (Clipboard.text && Clipboard.text.length > 0) {
                                sourceTextArea.text = Clipboard.text
                            } else {
                                remorsePopup.execute(qsTr("Clipboard is empty"), function() {})
                            }
                        }
                    }

                    Button {
                        text: qsTr("🌐 Fetch URL")
                        preferredWidth: Theme.buttonWidthExtraSmall
                        enabled: !agentBridge.agent_busy
                        onClicked: {
                            assistantPage.showUrlInput = !assistantPage.showUrlInput
                        }
                    }

                    Button {
                        text: qsTr("📁 Local File")
                        preferredWidth: Theme.buttonWidthExtraSmall
                        enabled: !agentBridge.agent_busy
                        onClicked: {
                            assistantPage.showFileInput = !assistantPage.showFileInput
                        }
                    }

                    IconButton {
                        visible: (typeof app !== "undefined" && app.sttEnabled !== undefined) ? app.sttEnabled : true
                        icon.source: assistantPage.isSpeechRecording ? "image://theme/icon-m-clear" : "image://theme/icon-m-mic"
                        highlighted: assistantPage.isSpeechRecording
                        enabled: !agentBridge.agent_busy && !assistantPage.isSpeechTranscribing
                        onClicked: {
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
                    }
                }

                // URL Input Row (expandable)
                Item {
                    width: parent.width - Theme.horizontalPageMargin * 2
                    height: assistantPage.showUrlInput ? (urlRow.height + Theme.paddingSmall) : 0
                    anchors.horizontalCenter: parent.horizontalCenter
                    visible: assistantPage.showUrlInput
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
                            label: qsTr("Web Page URL")
                            inputMethodHints: Qt.ImhUrlCharactersOnly
                            EnterKey.onClicked: fetchBtn.clicked()
                        }

                        Button {
                            id: fetchBtn
                            text: qsTr("Fetch")
                            preferredWidth: Theme.buttonWidthExtraSmall
                            enabled: urlField.text.trim().length > 0 && !agentBridge.agent_busy && !agentBridge.is_fetching
                            anchors.verticalCenter: urlField.verticalCenter
                            onClicked: {
                                agentBridge.fetch_url_content(urlField.text)
                            }
                        }
                    }
                }

                // File Path Input Row (expandable)
                Item {
                    width: parent.width - Theme.horizontalPageMargin * 2
                    height: assistantPage.showFileInput ? (fileRow.height + Theme.paddingSmall) : 0
                    anchors.horizontalCenter: parent.horizontalCenter
                    visible: assistantPage.showFileInput
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
                            label: qsTr("Local File Path")
                            EnterKey.onClicked: readFileBtn.clicked()
                        }

                        Button {
                            id: readFileBtn
                            text: qsTr("Load")
                            preferredWidth: Theme.buttonWidthExtraSmall
                            enabled: filePathField.text.trim().length > 0 && !agentBridge.agent_busy && !agentBridge.is_fetching
                            anchors.verticalCenter: filePathField.verticalCenter
                            onClicked: {
                                agentBridge.read_local_file(filePathField.text)
                            }
                        }
                    }
                }

                TextArea {
                    id: sourceTextArea
                    width: parent.width - Theme.horizontalPageMargin * 2
                    anchors.horizontalCenter: parent.horizontalCenter
                    placeholderText: qsTr("Paste external text, Markdown, notes, emails, transcripts, or articles here...")
                    label: qsTr("Source Text")
                    height: Math.max(Theme.itemSizeLarge * 2, implicitHeight)
                }

                // Options Section
                SectionHeader {
                    text: qsTr("Import Options")
                }

                TextField {
                    id: titleField
                    width: parent.width - Theme.horizontalPageMargin * 2
                    anchors.horizontalCenter: parent.horizontalCenter
                    label: qsTr("Note Title (Optional)")
                    placeholderText: qsTr("Auto-detected if left empty")
                    EnterKey.iconSource: "image://theme/icon-m-enter-close"
                    EnterKey.onClicked: focus = false
                }

                ComboBox {
                    id: modeComboBox
                    width: parent.width
                    label: qsTr("Conversion Mode")
                    currentIndex: 0

                    menu: ContextMenu {
                        MenuItem { text: qsTr("Full Document (Complete AsciiDoc)") }
                        MenuItem { text: qsTr("Summarize & Structure") }
                        MenuItem { text: qsTr("Extract Action Items / Checklists") }
                    }
                }

                TextField {
                    id: customPromptField
                    width: parent.width - Theme.horizontalPageMargin * 2
                    anchors.horizontalCenter: parent.horizontalCenter
                    label: qsTr("Custom Instructions (Optional)")
                    placeholderText: qsTr("e.g. Add syntax highlighting, keep concise")
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
                        text: agentBridge.agent_busy ? qsTr("Converting & Importing...") : qsTr("🚀 Convert & Import as Note")
                        enabled: !agentBridge.agent_busy && sourceTextArea.text.trim().length > 0
                        onClicked: {
                            assistantPage.applyConfig()
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
                    visible: agentBridge.agent_busy && agentBridge.streaming_text.length === 0 && currentTab === 1
                }

                // Live Streaming Progress Card
                Rectangle {
                    width: parent.width - Theme.horizontalPageMargin * 2
                    anchors.horizontalCenter: parent.horizontalCenter
                    height: streamCol.height + Theme.paddingMedium * 2
                    color: Theme.rgba(Theme.highlightBackgroundColor, 0.1)
                    radius: Theme.paddingSmall
                    border.color: Theme.rgba(Theme.highlightColor, 0.3)
                    border.width: 1
                    visible: agentBridge.agent_busy && agentBridge.streaming_text.length > 0 && currentTab === 1

                    Column {
                        id: streamCol
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.top: parent.top
                        anchors.margins: Theme.paddingMedium
                        spacing: Theme.paddingSmall

                        Row {
                            spacing: Theme.paddingSmall
                            Label {
                                text: qsTr("Converting...")
                                font.pixelSize: Theme.fontSizeExtraSmall
                                font.bold: true
                                color: Theme.highlightColor
                            }
                            BusyIndicator {
                                size: BusyIndicatorSize.ExtraSmall
                                running: true
                                anchors.verticalCenter: parent.verticalCenter
                            }
                        }

                        Label {
                            width: parent.width
                            text: agentBridge.streaming_text
                            font.pixelSize: Theme.fontSizeSmall
                            color: Theme.primaryColor
                            wrapMode: Text.Wrap
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
                                text: qsTr("Import Completed")
                                color: Theme.highlightColor
                                font.pixelSize: Theme.fontSizeMedium
                                font.bold: true
                                anchors.verticalCenter: parent.verticalCenter
                            }
                        }

                        Label {
                            text: qsTr("Created Note: ") + agentBridge.last_created_note
                            color: Theme.primaryColor
                            font.pixelSize: Theme.fontSizeMedium
                            font.bold: true
                            horizontalAlignment: Text.AlignHCenter
                            anchors.horizontalCenter: parent.horizontalCenter
                            wrapMode: Text.Wrap
                            width: parent.width
                        }

                        Button {
                            text: qsTr("📖 Open Created Note")
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

    RemorsePopup {
        id: remorsePopup
    }
}
