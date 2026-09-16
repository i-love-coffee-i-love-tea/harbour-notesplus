import QtQuick 2.6
import Sailfish.Silica 1.0
import harbour.notesplus 1.0
import "../components"
import "../js/ThemeColors.js" as TC

Page {
    id: aiImportPage
    allowedOrientations: Orientation.All

    property bool showUrlInput: false
    property bool showFileInput: false
    property bool showCustomPromptTips: false

    // Speech capture state from SpeechBridge
    property bool isSpeechRecording: typeof speechBridge !== "undefined" && speechBridge && speechBridge.is_recording
    property bool isSpeechTranscribing: typeof speechBridge !== "undefined" && speechBridge && speechBridge.is_transcribing
    property real liveAudioLevel: (typeof speechBridge !== "undefined" && speechBridge && aiImportPage.isSpeechRecording) ? speechBridge.audio_level : 0.0
    property var liveWaveform: {
        if (typeof speechBridge !== "undefined" && speechBridge && speechBridge.waveform_json && aiImportPage.isSpeechRecording) {
            try {
                return JSON.parse(speechBridge.waveform_json) || [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
            } catch (e) {
                return [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
            }
        }
        return [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
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
            cancelActiveRecording()
        }
    }

    property string pendingTranscription: (typeof speechBridge !== "undefined" && speechBridge) ? speechBridge.last_transcription : ""
    onPendingTranscriptionChanged: {
        if (aiImportPage.status !== PageStatus.Active) return
        var trans = pendingTranscription
        if (trans && trans.trim().length > 0) {
            var clean = trans.trim()
            if (sourceTextArea.text.length > 0) {
                sourceTextArea.text = sourceTextArea.text + " " + clean
            } else {
                sourceTextArea.text = clean
            }
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

    AgentBridge {
        id: agentBridge

        Component.onCompleted: {
            aiImportPage.applyConfig()
        }
    }

    Connections {
        target: agentBridge

        onFetch_completed: {
            sourceTextArea.text = result
            aiImportPage.showUrlInput = false
            aiImportPage.showFileInput = false
        }

        onFetch_error: {
            remorsePopup.execute(message, function() {})
        }

        onResponse_finished: {
            bridge.load_main_page_data()
        }

        onError_occurred: {
            remorsePopup.execute("AI Error: " + message, function() {})
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
        }

        Column {
            id: mainColumn
            width: parent.width
            spacing: Theme.paddingMedium

            PageHeader {
                title: qsTr("AI Import")
                description: qsTr("Convert external sources to AsciiDoc")
            }

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
                        aiImportPage.showUrlInput = !aiImportPage.showUrlInput
                    }
                }

                Button {
                    text: qsTr("📁 Local File")
                    preferredWidth: Theme.buttonWidthExtraSmall
                    enabled: !agentBridge.agent_busy
                    onClicked: {
                        aiImportPage.showFileInput = !aiImportPage.showFileInput
                    }
                }
            }

            // URL Input Row (expandable)
            Item {
                width: parent.width - Theme.horizontalPageMargin * 2
                height: aiImportPage.showUrlInput ? (urlRow.height + Theme.paddingSmall) : 0
                anchors.horizontalCenter: parent.horizontalCenter
                visible: aiImportPage.showUrlInput
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
                        onClicked: agentBridge.fetch_url_content(urlField.text)
                    }
                }
            }

            // File Path Input Row (expandable)
            Item {
                width: parent.width - Theme.horizontalPageMargin * 2
                height: aiImportPage.showFileInput ? (fileRow.height + Theme.paddingSmall) : 0
                anchors.horizontalCenter: parent.horizontalCenter
                visible: aiImportPage.showFileInput
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
                        onClicked: agentBridge.read_local_file(filePathField.text)
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

            VoiceInputBar {
                visible: (typeof app !== "undefined" && app.sttEnabled !== undefined) ? app.sttEnabled : true
                isSpeechRecording: aiImportPage.isSpeechRecording
                isSpeechTranscribing: aiImportPage.isSpeechTranscribing
                liveAudioLevel: aiImportPage.liveAudioLevel
                liveWaveform: aiImportPage.liveWaveform
                micEnabled: !agentBridge.agent_busy && !aiImportPage.isSpeechTranscribing
                onToggleMic: aiImportPage.handleMicClick()
            }

            // Options Section
            SectionHeader {
                text: qsTr("Import Options")
            }

            DescribedTextField {
                id: titleField
                width: parent.width - Theme.horizontalPageMargin * 2
                anchors.horizontalCenter: parent.horizontalCenter
                fieldLabel: qsTr("Note Title")
                fieldDescription: qsTr("Leave empty to auto-detect from content")
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

            Row {
                width: parent.width - Theme.horizontalPageMargin * 2
                anchors.horizontalCenter: parent.horizontalCenter
                spacing: 0

                DescribedTextField {
                    id: customPromptField
                    width: parent.width - tipsBtn.width
                    fieldLabel: qsTr("Custom Instructions")
                    fieldDescription: qsTr("Steer the conversion with formatting, tone, or scope preferences")
                }

                IconButton {
                    id: tipsBtn
                    icon.source: "image://theme/icon-m-about"
                    anchors.verticalCenter: customPromptField.verticalCenter
                    onClicked: aiImportPage.showCustomPromptTips = !aiImportPage.showCustomPromptTips
                }
            }

            Column {
                width: parent.width - Theme.horizontalPageMargin * 2
                anchors.horizontalCenter: parent.horizontalCenter
                spacing: Theme.paddingSmall
                visible: aiImportPage.showCustomPromptTips
                clip: true
                height: aiImportPage.showCustomPromptTips ? implicitHeight : 0
                Behavior on height { NumberAnimation { duration: 150 } }

                Label {
                    width: parent.width
                    text: qsTr("Tip: Add extra instructions to steer the conversion. Examples:")
                    font.pixelSize: Theme.fontSizeExtraSmall
                    color: Theme.highlightColor
                    wrapMode: Text.Wrap
                }
                Label {
                    width: parent.width
                    text: "\u2022 " + qsTr("Only extract action items, skip the discussion")
                    font.pixelSize: Theme.fontSizeExtraSmall
                    color: Theme.secondaryColor
                    wrapMode: Text.Wrap
                }
                Label {
                    width: parent.width
                    text: "\u2022 " + qsTr("Use description lists instead of bullet points for definitions")
                    font.pixelSize: Theme.fontSizeExtraSmall
                    color: Theme.secondaryColor
                    wrapMode: Text.Wrap
                }
                Label {
                    width: parent.width
                    text: "\u2022 " + qsTr("Write in first person as if journaling")
                    font.pixelSize: Theme.fontSizeExtraSmall
                    color: Theme.secondaryColor
                    wrapMode: Text.Wrap
                }
                Label {
                    width: parent.width
                    text: "\u2022 " + qsTr("Organize as a FAQ with Q: and A: headings")
                    font.pixelSize: Theme.fontSizeExtraSmall
                    color: Theme.secondaryColor
                    wrapMode: Text.Wrap
                }
                Label {
                    width: parent.width
                    text: "\u2022 " + qsTr("Tag with :keywords: machine-learning, python")
                    font.pixelSize: Theme.fontSizeExtraSmall
                    color: Theme.secondaryColor
                    wrapMode: Text.Wrap
                }
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
                        var mode = "convert_full"
                        if (modeComboBox.currentIndex === 1) mode = "summarize"
                        else if (modeComboBox.currentIndex === 2) mode = "action_items"

                        aiImportPage.applyConfig()
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
                visible: agentBridge.agent_busy && agentBridge.streaming_text.length === 0
            }

            // Live Streaming Progress Card
            InfoCard {
                visible: agentBridge.agent_busy && agentBridge.streaming_text.length > 0

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

            // Result Card when a note is created
            InfoCard {
                visible: agentBridge.last_created_note.length > 0 && !agentBridge.agent_busy

                Row {
                    spacing: Theme.paddingSmall
                    anchors.horizontalCenter: parent.horizontalCenter

                    Label {
                        text: "✓"
                        color: TC.kStatusGreen
                        font.pixelSize: Theme.fontSizeLarge
                        font.bold: true
                        anchors.verticalCenter: parent.verticalCenter
                    }

                    Label {
                        text: qsTr("Import Completed")
                        color: Theme.primaryColor
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
                        pageStack.push(Qt.resolvedUrl("PageView.qml"), {
                            pageName: agentBridge.last_created_note
                        })
                        bridge.load_page(agentBridge.last_created_note)
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
                        border.color: modelData.role === "assistant" ? Theme.rgba(Theme.primaryColor, 0.2) : "transparent"
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
                                color: Theme.primaryColor
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
