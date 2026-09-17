import QtQuick 2.6
import Sailfish.Silica 1.0
import "../js/ThemeColors.js" as TC

Rectangle {
    id: promptBarCard
    width: parent.width - Theme.horizontalPageMargin * 2
    anchors.horizontalCenter: parent.horizontalCenter
    height: contentColumn.height + Theme.paddingMedium * 2
    radius: Theme.paddingSmall
    color: promptField.activeFocus ? Theme.rgba(Theme.highlightBackgroundColor, 0.12) : Theme.rgba(Theme.highlightBackgroundColor, 0.06)
    border.color: promptField.activeFocus ? Theme.rgba(Theme.highlightColor, 0.45) : Theme.rgba(Theme.primaryColor, 0.18)
    border.width: 1

    property alias text: promptField.text
    property alias placeholderText: promptField.placeholderText
    property bool agentBusy: false
    property var attachedNotes: []
    property string contextFilename: ""
    property string contextContent: ""
    property string extraContext: ""

    property bool showInstructions: false
    property var customInstructions: (typeof app !== "undefined" && app.customAiInstructions) ? app.customAiInstructions : []
    readonly property int instructionsCount: (customInstructions && customInstructions.length) ? customInstructions.length : 0

    readonly property var effectiveNotes: {
        if (attachedNotes && attachedNotes.length > 0) {
            return attachedNotes
        }
        if (contextFilename.length > 0 || contextContent.length > 0) {
            return [{
                filename: contextFilename,
                title: contextFilename,
                content: contextContent
            }]
        }
        return []
    }

    readonly property bool hasNote: effectiveNotes.length > 0
    readonly property bool hasClipboard: (extraContext.length > 0)
    readonly property bool hasContext: hasNote || hasClipboard
    property bool hasContextOrInput: hasContext || (promptField.text && promptField.text.trim().length > 0)

    property bool isSpeechRecording: false
    property bool isSpeechTranscribing: false
    property real liveAudioLevel: 0.0
    property var liveWaveform: [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
    property bool sttEnabled: true

    signal submitPrompt(string text)
    signal toggleMic()
    signal cancelRecording()
    signal attachNoteRequested()
    signal detachNoteRequested(string filename, int index)
    signal attachClipboardRequested()
    signal detachExtraContextRequested()
    signal clearAllContextRequested()
    signal instructionSelected(var instructionItem)
    signal editInstructionRequested(var instructionItem)

    Column {
        id: contentColumn
        anchors {
            left: parent.left
            right: parent.right
            top: parent.top
            margins: Theme.paddingMedium
        }
        spacing: Theme.paddingSmall

        // 1. Attached Notes Chips (Repeater for 1 or multiple attached notes)
        Repeater {
            model: promptBarCard.effectiveNotes

            delegate: Rectangle {
                id: noteChip
                width: parent.width
                height: noteRow.height + Theme.paddingSmall * 2
                color: Theme.rgba(Theme.highlightBackgroundColor, 0.15)
                radius: Theme.paddingSmall / 2
                border.color: Theme.rgba(Theme.highlightColor, 0.35)
                border.width: 1

                Row {
                    id: noteRow
                    anchors {
                        left: parent.left
                        right: parent.right
                        verticalCenter: parent.verticalCenter
                        margins: Theme.paddingSmall
                    }
                    spacing: Theme.paddingSmall

                    Icon {
                        source: "image://theme/icon-m-document"
                        width: Theme.iconSizeSmall
                        height: Theme.iconSizeSmall
                        color: Theme.highlightColor
                        anchors.verticalCenter: parent.verticalCenter
                    }

                    Column {
                        width: parent.width - Theme.iconSizeSmall - detachNoteBtn.width - Theme.paddingSmall * 2
                        anchors.verticalCenter: parent.verticalCenter

                        Label {
                            text: (modelData.title && modelData.title.length > 0) ? modelData.title : (modelData.filename && modelData.filename.length > 0 ? modelData.filename : qsTr("Active Note"))
                            font.pixelSize: Theme.fontSizeExtraSmall
                            font.bold: true
                            color: Theme.primaryColor
                            truncationMode: TruncationMode.Fade
                            width: parent.width
                        }

                        Label {
                            text: {
                                var cnt = modelData.content || ""
                                var chars = cnt.length
                                var lines = cnt.length > 0 ? cnt.split("\n").length : 0
                                return qsTr("Note content: %1 chars, %2 lines").arg(chars).arg(lines)
                            }
                            font.pixelSize: Theme.fontSizeTiny
                            color: Theme.secondaryColor
                            truncationMode: TruncationMode.Fade
                            width: parent.width
                        }
                    }

                    IconButton {
                        id: detachNoteBtn
                        icon.source: "image://theme/icon-m-clear"
                        icon.width: Theme.iconSizeSmall
                        icon.height: Theme.iconSizeSmall
                        width: Theme.itemSizeExtraSmall
                        height: Theme.itemSizeExtraSmall
                        anchors.verticalCenter: parent.verticalCenter
                        onClicked: promptBarCard.detachNoteRequested(modelData.filename || "", index)
                    }
                }
            }
        }

        // 2. Attached Clipboard Chip (when clipboard / extra context is present)
        Rectangle {
            id: clipboardChip
            width: parent.width
            height: extraRow.height + Theme.paddingSmall * 2
            visible: promptBarCard.hasClipboard
            color: Theme.rgba(Theme.highlightBackgroundColor, 0.15)
            radius: Theme.paddingSmall / 2
            border.color: Theme.rgba(Theme.highlightColor, 0.35)
            border.width: 1

            Row {
                id: extraRow
                anchors {
                    left: parent.left
                    right: parent.right
                    verticalCenter: parent.verticalCenter
                    margins: Theme.paddingSmall
                }
                spacing: Theme.paddingSmall

                Icon {
                    source: "image://theme/icon-m-clipboard"
                    width: Theme.iconSizeSmall
                    height: Theme.iconSizeSmall
                    color: Theme.highlightColor
                    anchors.verticalCenter: parent.verticalCenter
                }

                Column {
                    width: parent.width - Theme.iconSizeSmall - detachExtraBtn.width - Theme.paddingSmall * 2
                    anchors.verticalCenter: parent.verticalCenter

                    Label {
                        text: qsTr("Clipboard Context")
                        font.pixelSize: Theme.fontSizeExtraSmall
                        font.bold: true
                        color: Theme.primaryColor
                        truncationMode: TruncationMode.Fade
                        width: parent.width
                    }

                    Label {
                        text: {
                            var chars = promptBarCard.extraContext.length
                            var lines = promptBarCard.extraContext.length > 0 ? promptBarCard.extraContext.split("\n").length : 0
                            return qsTr("%1 chars, %2 lines").arg(chars).arg(lines)
                        }
                        font.pixelSize: Theme.fontSizeTiny
                        color: Theme.secondaryColor
                        truncationMode: TruncationMode.Fade
                        width: parent.width
                    }
                }

                IconButton {
                    id: detachExtraBtn
                    icon.source: "image://theme/icon-m-clear"
                    icon.width: Theme.iconSizeSmall
                    icon.height: Theme.iconSizeSmall
                    width: Theme.itemSizeExtraSmall
                    height: Theme.itemSizeExtraSmall
                    anchors.verticalCenter: parent.verticalCenter
                    onClicked: promptBarCard.detachExtraContextRequested()
                }
            }
        }

        // 3. Prompt Text Input Area
        TextArea {
            id: promptField
            width: parent.width
            placeholderText: qsTr("Ask anything about your notes or request changes...")
            background: null
            labelVisible: false
            height: Math.max(Theme.itemSizeMedium, implicitHeight)
        }

        // 4. Voice Recording / Transcribing Live Indicator (when active)
        Rectangle {
            width: parent.width
            height: (promptBarCard.isSpeechRecording || promptBarCard.isSpeechTranscribing) ? Theme.itemSizeExtraSmall : 0
            color: Theme.rgba(Theme.highlightBackgroundColor, 0.15)
            radius: Theme.paddingSmall / 2
            border.color: (promptBarCard.isSpeechRecording && promptBarCard.liveAudioLevel > 0.06) ?
                          Theme.rgba(Theme.highlightColor, 0.6) : Theme.rgba(Theme.primaryColor, 0.2)
            border.width: 1
            visible: promptBarCard.isSpeechRecording || promptBarCard.isSpeechTranscribing
            clip: true

            Behavior on height { NumberAnimation { duration: 150 } }
            Behavior on border.color { ColorAnimation { duration: 100 } }

            Row {
                anchors.centerIn: parent
                spacing: Theme.paddingMedium

                BusyIndicator {
                    size: BusyIndicatorSize.ExtraSmall
                    running: promptBarCard.isSpeechTranscribing
                    visible: promptBarCard.isSpeechTranscribing
                    anchors.verticalCenter: parent.verticalCenter
                }

                Row {
                    spacing: 3
                    anchors.verticalCenter: parent.verticalCenter
                    visible: promptBarCard.isSpeechRecording

                    Repeater {
                        model: 7
                        Rectangle {
                            width: 3
                            readonly property real barVal: (promptBarCard.liveWaveform && promptBarCard.liveWaveform.length > index) ? promptBarCard.liveWaveform[index] : 0.0
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
                        if (promptBarCard.isSpeechTranscribing) {
                            return qsTr("Transcribing speech...")
                        }
                        if (promptBarCard.liveAudioLevel > 0.06) {
                            var percent = Math.round(promptBarCard.liveAudioLevel * 100)
                            return qsTr("Hearing voice (%1%)... Tap mic to finish").arg(percent)
                        }
                        return qsTr("Listening... Speak into microphone")
                    }
                    color: Theme.primaryColor
                    font.pixelSize: Theme.fontSizeExtraSmall
                    anchors.verticalCenter: parent.verticalCenter
                }

                Label {
                    text: qsTr("Cancel")
                    color: Theme.secondaryColor
                    font.pixelSize: Theme.fontSizeExtraSmall
                    visible: promptBarCard.isSpeechRecording
                    anchors.verticalCenter: parent.verticalCenter
                    MouseArea {
                        anchors.fill: parent
                        anchors.margins: -Theme.paddingSmall
                        onClicked: promptBarCard.cancelRecording()
                    }
                }
            }
        }

        // 5. Quick Instructions Section (Collapsible within the composer card)
        Column {
            id: instructionsSection
            width: parent.width
            spacing: Theme.paddingSmall
            visible: promptBarCard.showInstructions

            // Section Divider & Header
            Item {
                width: parent.width
                height: Theme.itemSizeExtraSmall

                Rectangle {
                    anchors.top: parent.top
                    width: parent.width
                    height: 1
                    color: Theme.rgba(Theme.primaryColor, 0.15)
                }

                Row {
                    anchors.left: parent.left
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: Theme.paddingSmall

                    Icon {
                        source: "image://theme/icon-m-developer-mode"
                        width: Theme.iconSizeSmall
                        height: Theme.iconSizeSmall
                        color: Theme.highlightColor
                        anchors.verticalCenter: parent.verticalCenter
                    }

                    Label {
                        text: qsTr("Quick Instructions")
                        font.bold: true
                        font.pixelSize: Theme.fontSizeSmall
                        color: Theme.highlightColor
                        anchors.verticalCenter: parent.verticalCenter
                    }

                    Label {
                        text: "(%1)".arg(promptBarCard.instructionsCount)
                        font.pixelSize: Theme.fontSizeExtraSmall
                        color: Theme.secondaryColor
                        anchors.verticalCenter: parent.verticalCenter
                    }
                }

                IconButton {
                    anchors.right: parent.right
                    anchors.verticalCenter: parent.verticalCenter
                    icon.source: "image://theme/icon-m-clear"
                    icon.width: Theme.iconSizeSmall
                    icon.height: Theme.iconSizeSmall
                    width: Theme.itemSizeExtraSmall
                    height: Theme.itemSizeExtraSmall
                    onClicked: promptBarCard.showInstructions = false
                }
            }

            // Instructions Button Grid
            Grid {
                id: instructionsGrid
                width: parent.width
                columns: width > 600 ? 3 : 2
                spacing: Theme.paddingSmall

                readonly property real itemWidth: Math.floor((width - (columns - 1) * spacing) / columns)

                Repeater {
                    model: promptBarCard.customInstructions

                    delegate: BackgroundItem {
                        id: tplBtn
                        width: instructionsGrid.itemWidth
                        height: Theme.itemSizeExtraSmall
                        enabled: !promptBarCard.agentBusy
                        opacity: (!promptBarCard.agentBusy) ? 1.0 : 0.4

                        readonly property bool isVendored: (typeof app !== "undefined" && app.isDefaultAiInstruction) ?
                                                               app.isDefaultAiInstruction(modelData.id) : false

                        Rectangle {
                            anchors.fill: parent
                            radius: Theme.paddingSmall / 2
                            color: tplBtn.highlighted ? Theme.rgba(Theme.highlightBackgroundColor, 0.45) :
                                   (tplBtn.isVendored ? Theme.rgba(Theme.highlightBackgroundColor, 0.16) : Theme.rgba(Theme.primaryColor, 0.05))
                            border.color: tplBtn.highlighted ? Theme.highlightColor :
                                          (tplBtn.isVendored ? Theme.rgba(Theme.highlightColor, 0.35) : Theme.rgba(Theme.primaryColor, 0.2))
                            border.width: 1

                            Row {
                                anchors {
                                    left: parent.left
                                    right: parent.right
                                    verticalCenter: parent.verticalCenter
                                    margins: Theme.paddingSmall
                                }
                                spacing: Theme.paddingSmall

                                Icon {
                                    source: modelData.icon ? (modelData.icon.indexOf("image://") === 0 ? modelData.icon : ("image://theme/" + modelData.icon)) : "image://theme/icon-m-note"
                                    width: Theme.iconSizeSmall
                                    height: Theme.iconSizeSmall
                                    color: tplBtn.highlighted ? Theme.highlightColor :
                                           (tplBtn.isVendored ? Theme.primaryColor : Theme.secondaryColor)
                                    anchors.verticalCenter: parent.verticalCenter
                                }

                                Label {
                                    text: modelData.buttonText || modelData.title || ""
                                    font.pixelSize: Theme.fontSizeExtraSmall
                                    font.bold: true
                                    color: tplBtn.highlighted ? Theme.highlightColor : Theme.primaryColor
                                    anchors.verticalCenter: parent.verticalCenter
                                    width: parent.width - Theme.iconSizeSmall - Theme.paddingSmall
                                    truncationMode: TruncationMode.Fade
                                }
                            }
                        }

                        onClicked: {
                            promptBarCard.instructionSelected(modelData)
                        }

                        onPressAndHold: {
                            promptBarCard.editInstructionRequested(modelData)
                        }
                    }
                }
            }

            // Tip / Instruction Hint
            Label {
                width: parent.width
                text: qsTr("Tip: Tap to run action, hold to view or edit prompt.")
                font.pixelSize: Theme.fontSizeExtraSmall - 2
                color: Theme.secondaryColor
                wrapMode: Text.Wrap
            }
        }

        // 6. Bottom Toolbar Controls Row (Note, Clipboard, Instructions on left; Voice & Send on right)
        Item {
            width: parent.width
            height: Theme.itemSizeExtraSmall

            // Left: Attachment & Instruction Action Buttons (Icon + Text)
            Row {
                anchors.left: parent.left
                anchors.verticalCenter: parent.verticalCenter
                spacing: Theme.paddingSmall

                // 1. Attach Note Button
                BackgroundItem {
                    id: attachNoteBtn
                    height: Theme.itemSizeExtraSmall
                    width: noteBtnRow.width + Theme.paddingSmall * 2
                    highlightedColor: "transparent"

                    Rectangle {
                        anchors.fill: parent
                        radius: Theme.paddingSmall / 2
                        color: (attachNoteBtn.highlighted || promptBarCard.hasNote) ? Theme.rgba(Theme.highlightBackgroundColor, 0.2) : "transparent"
                        border.color: (attachNoteBtn.highlighted || promptBarCard.hasNote) ? Theme.rgba(Theme.highlightColor, 0.4) : "transparent"
                        border.width: 1
                        visible: attachNoteBtn.highlighted || promptBarCard.hasNote
                    }

                    Row {
                        id: noteBtnRow
                        anchors.centerIn: parent
                        spacing: Theme.paddingSmall / 2

                        Icon {
                            source: "image://theme/icon-m-document"
                            width: Theme.iconSizeSmall
                            height: Theme.iconSizeSmall
                            color: (attachNoteBtn.highlighted || promptBarCard.hasNote) ? Theme.highlightColor : Theme.primaryColor
                            anchors.verticalCenter: parent.verticalCenter
                        }

                        Label {
                            text: qsTr("Note")
                            font.pixelSize: Theme.fontSizeExtraSmall
                            font.bold: promptBarCard.hasNote
                            color: (attachNoteBtn.highlighted || promptBarCard.hasNote) ? Theme.highlightColor : Theme.primaryColor
                            anchors.verticalCenter: parent.verticalCenter
                        }
                    }

                    onClicked: promptBarCard.attachNoteRequested()
                }

                // 2. Attach Clipboard Button
                BackgroundItem {
                    id: attachClipboardBtn
                    height: Theme.itemSizeExtraSmall
                    width: clipBtnRow.width + Theme.paddingSmall * 2
                    highlightedColor: "transparent"

                    Rectangle {
                        anchors.fill: parent
                        radius: Theme.paddingSmall / 2
                        color: (attachClipboardBtn.highlighted || promptBarCard.hasClipboard) ? Theme.rgba(Theme.highlightBackgroundColor, 0.2) : "transparent"
                        border.color: (attachClipboardBtn.highlighted || promptBarCard.hasClipboard) ? Theme.rgba(Theme.highlightColor, 0.4) : "transparent"
                        border.width: 1
                        visible: attachClipboardBtn.highlighted || promptBarCard.hasClipboard
                    }

                    Row {
                        id: clipBtnRow
                        anchors.centerIn: parent
                        spacing: Theme.paddingSmall / 2

                        Icon {
                            source: "image://theme/icon-m-clipboard"
                            width: Theme.iconSizeSmall
                            height: Theme.iconSizeSmall
                            color: (attachClipboardBtn.highlighted || promptBarCard.hasClipboard) ? Theme.highlightColor : Theme.primaryColor
                            anchors.verticalCenter: parent.verticalCenter
                        }

                        Label {
                            text: qsTr("Clipboard")
                            font.pixelSize: Theme.fontSizeExtraSmall
                            font.bold: promptBarCard.hasClipboard
                            color: (attachClipboardBtn.highlighted || promptBarCard.hasClipboard) ? Theme.highlightColor : Theme.primaryColor
                            anchors.verticalCenter: parent.verticalCenter
                        }
                    }

                    onClicked: promptBarCard.attachClipboardRequested()
                }

                // 3. Quick Instructions Toggle Button
                BackgroundItem {
                    id: toggleInstructionsBtn
                    height: Theme.itemSizeExtraSmall
                    width: instBtnRow.width + Theme.paddingSmall * 2
                    highlightedColor: "transparent"

                    Rectangle {
                        anchors.fill: parent
                        radius: Theme.paddingSmall / 2
                        color: (toggleInstructionsBtn.highlighted || promptBarCard.showInstructions) ? Theme.rgba(Theme.highlightBackgroundColor, 0.2) : "transparent"
                        border.color: (toggleInstructionsBtn.highlighted || promptBarCard.showInstructions) ? Theme.rgba(Theme.highlightColor, 0.4) : "transparent"
                        border.width: 1
                        visible: toggleInstructionsBtn.highlighted || promptBarCard.showInstructions
                    }

                    Row {
                        id: instBtnRow
                        anchors.centerIn: parent
                        spacing: Theme.paddingSmall / 2

                        Icon {
                            source: "image://theme/icon-m-developer-mode"
                            width: Theme.iconSizeSmall
                            height: Theme.iconSizeSmall
                            color: (toggleInstructionsBtn.highlighted || promptBarCard.showInstructions) ? Theme.highlightColor : Theme.primaryColor
                            anchors.verticalCenter: parent.verticalCenter
                        }

                        Label {
                            text: qsTr("Instructions")
                            font.pixelSize: Theme.fontSizeExtraSmall
                            font.bold: promptBarCard.showInstructions
                            color: (toggleInstructionsBtn.highlighted || promptBarCard.showInstructions) ? Theme.highlightColor : Theme.primaryColor
                            anchors.verticalCenter: parent.verticalCenter
                        }
                    }

                    onClicked: {
                        promptBarCard.showInstructions = !promptBarCard.showInstructions
                    }
                }
            }

            // Right: Voice and Send Controls
            Row {
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                spacing: Theme.paddingSmall / 2

                // Mic Button with Audio Level Halo
                Item {
                    width: micBtn.width
                    height: micBtn.height
                    visible: promptBarCard.sttEnabled
                    opacity: (!promptBarCard.isSpeechTranscribing) ? 1.0 : Theme.opacityFaint

                    Rectangle {
                        anchors.centerIn: parent
                        width: parent.width + Theme.paddingSmall + Math.round(promptBarCard.liveAudioLevel * 28)
                        height: parent.height + Theme.paddingSmall + Math.round(promptBarCard.liveAudioLevel * 28)
                        radius: width / 2
                        color: (promptBarCard.liveAudioLevel > 0.06) ? TC.kVoiceActive : TC.kVoiceInactive
                        opacity: promptBarCard.isSpeechRecording ? Math.min(0.85, 0.25 + promptBarCard.liveAudioLevel * 0.6) : 0.0
                        visible: promptBarCard.isSpeechRecording

                        Behavior on width { NumberAnimation { duration: 60 } }
                        Behavior on height { NumberAnimation { duration: 60 } }
                        Behavior on opacity { NumberAnimation { duration: 60 } }
                        Behavior on color { ColorAnimation { duration: 100 } }
                    }

                    IconButton {
                        id: micBtn
                        anchors.centerIn: parent
                        icon.source: promptBarCard.isSpeechRecording ? "image://theme/icon-m-clear" : "image://theme/icon-m-mic"
                        icon.width: Theme.iconSizeSmall
                        icon.height: Theme.iconSizeSmall
                        width: Theme.itemSizeExtraSmall
                        height: Theme.itemSizeExtraSmall
                        highlighted: promptBarCard.isSpeechRecording
                        enabled: !promptBarCard.isSpeechTranscribing
                        onClicked: promptBarCard.toggleMic()
                    }
                }

                // Send Prompt Button
                IconButton {
                    id: sendBtn
                    icon.source: "image://theme/icon-m-send"
                    icon.width: Theme.iconSizeSmall
                    icon.height: Theme.iconSizeSmall
                    width: Theme.itemSizeExtraSmall
                    height: Theme.itemSizeExtraSmall
                    enabled: !promptBarCard.agentBusy && !promptBarCard.isSpeechTranscribing && promptField.text.trim().length > 0
                    onClicked: {
                        if (promptField.text.trim().length > 0) {
                            promptBarCard.submitPrompt(promptField.text)
                        }
                    }
                }
            }
        }
    }
}
