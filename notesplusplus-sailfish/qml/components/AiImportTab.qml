import QtQuick 2.0
import Sailfish.Silica 1.0

Column {
    id: importTab
    width: parent.width
    spacing: Theme.paddingMedium

    property alias sourceText: sourceTextArea.text
    property alias noteTitle: titleField.text
    property alias customPrompt: customPromptField.text
    property alias urlText: urlField.text
    property alias filePathText: filePathField.text

    property bool showUrlInput: false
    property bool showFileInput: false
    property bool agentBusy: false
    property bool isFetching: false
    property string streamingText: ""
    property string lastCreatedNote: ""
    property string messagesJson: ""
    property bool isSpeechRecording: false
    property bool isSpeechTranscribing: false
    property bool sttEnabled: true

    signal convertRequested(string sourceText, string title, string mode, string customPrompt)
    signal fetchUrlRequested(string url)
    signal readFileRequested(string filePath)
    signal toggleMic()
    signal openCreatedNote(string noteTitle)
    signal clipboardPasted()

    SectionHeader {
        text: qsTr("Source Content")
    }

    Row {
        anchors.horizontalCenter: parent.horizontalCenter
        spacing: Theme.paddingSmall

        Button {
            text: qsTr("📋 Paste Clipboard")
            preferredWidth: Theme.buttonWidthExtraSmall
            enabled: !importTab.agentBusy
            onClicked: importTab.clipboardPasted()
        }

        Button {
            text: qsTr("🌐 Fetch URL")
            preferredWidth: Theme.buttonWidthExtraSmall
            enabled: !importTab.agentBusy
            onClicked: {
                importTab.showUrlInput = !importTab.showUrlInput
            }
        }

        Button {
            text: qsTr("📁 Local File")
            preferredWidth: Theme.buttonWidthExtraSmall
            enabled: !importTab.agentBusy
            onClicked: {
                importTab.showFileInput = !importTab.showFileInput
            }
        }

        IconButton {
            visible: importTab.sttEnabled
            icon.source: importTab.isSpeechRecording ? "image://theme/icon-m-clear" : "image://theme/icon-m-mic"
            highlighted: importTab.isSpeechRecording
            enabled: !importTab.agentBusy && !importTab.isSpeechTranscribing
            onClicked: importTab.toggleMic()
        }
    }

    // URL Input Row (expandable)
    Item {
        width: parent.width - Theme.horizontalPageMargin * 2
        height: importTab.showUrlInput ? (urlRow.height + Theme.paddingSmall) : 0
        anchors.horizontalCenter: parent.horizontalCenter
        visible: importTab.showUrlInput
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
                enabled: urlField.text.trim().length > 0 && !importTab.agentBusy && !importTab.isFetching
                anchors.verticalCenter: urlField.verticalCenter
                onClicked: importTab.fetchUrlRequested(urlField.text)
            }
        }
    }

    // File Path Input Row (expandable)
    Item {
        width: parent.width - Theme.horizontalPageMargin * 2
        height: importTab.showFileInput ? (fileRow.height + Theme.paddingSmall) : 0
        anchors.horizontalCenter: parent.horizontalCenter
        visible: importTab.showFileInput
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
                enabled: filePathField.text.trim().length > 0 && !importTab.agentBusy && !importTab.isFetching
                anchors.verticalCenter: filePathField.verticalCenter
                onClicked: importTab.readFileRequested(filePathField.text)
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
            text: importTab.agentBusy ? qsTr("Converting & Importing...") : qsTr("🚀 Convert & Import as Note")
            enabled: !importTab.agentBusy && sourceTextArea.text.trim().length > 0
            onClicked: {
                var mode = "convert_full"
                if (modeComboBox.currentIndex === 1) mode = "summarize"
                else if (modeComboBox.currentIndex === 2) mode = "action_items"

                importTab.convertRequested(
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
        running: importTab.agentBusy
        visible: importTab.agentBusy && importTab.streamingText.length === 0
    }

    // Live Streaming Progress Card
    InfoCard {
        visible: importTab.agentBusy && importTab.streamingText.length > 0

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
            text: importTab.streamingText
            font.pixelSize: Theme.fontSizeSmall
            color: Theme.primaryColor
            wrapMode: Text.Wrap
        }
    }

    // Result Card when a note is created
    InfoCard {
        visible: importTab.lastCreatedNote.length > 0 && !importTab.agentBusy

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
            text: qsTr("Created Note: ") + importTab.lastCreatedNote
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
            onClicked: importTab.openCreatedNote(importTab.lastCreatedNote)
        }
    }

    // Conversational Output Timeline
    Column {
        width: parent.width - Theme.horizontalPageMargin * 2
        anchors.horizontalCenter: parent.horizontalCenter
        spacing: Theme.paddingMedium
        visible: !importTab.agentBusy

        Repeater {
            model: {
                try {
                    var list = JSON.parse(importTab.messagesJson)
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
