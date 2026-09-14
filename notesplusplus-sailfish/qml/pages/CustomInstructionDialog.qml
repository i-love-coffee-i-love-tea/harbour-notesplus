import QtQuick 2.0
import Sailfish.Silica 1.0

Dialog {
    id: customInstructionDialog
    allowedOrientations: Orientation.All

    property string instructionId: ""
    property string initialButtonText: ""
    property string initialIcon: "icon-m-note"
    property string initialInstruction: ""
    property bool isEdit: false

    readonly property var defaultInstructionItem: {
        if (typeof app !== "undefined" && app.defaultCustomAiInstructions) {
            for (var i = 0; i < app.defaultCustomAiInstructions.length; i++) {
                if (app.defaultCustomAiInstructions[i].id === customInstructionDialog.instructionId) {
                    return app.defaultCustomAiInstructions[i]
                }
            }
        }
        return null
    }
    readonly property bool isStandardInstruction: defaultInstructionItem !== null

    property string selectedIcon: initialIcon.length > 0 ? initialIcon : (defaultInstructionItem ? defaultInstructionItem.icon : "icon-m-note")
    readonly property string effectiveInitialInstruction: {
        if (initialInstruction.length > 0) return initialInstruction
        if (defaultInstructionItem && defaultInstructionItem.instruction) {
            return defaultInstructionItem.instruction
        }
        return ""
    }
    readonly property string effectiveInitialButtonText: {
        if (initialButtonText.length > 0) return initialButtonText
        if (defaultInstructionItem && defaultInstructionItem.buttonText) {
            return defaultInstructionItem.buttonText
        }
        return ""
    }

    // Quick popular icons
    readonly property var popularIcons: [
        "icon-m-note",
        "icon-m-edit",
        "icon-m-document",
        "icon-m-favorite",
        "icon-m-chat",
        "icon-m-search",
        "icon-m-select-all",
        "icon-m-share",
        "icon-m-light-contrast",
        "icon-m-levels",
        "icon-m-website",
        "icon-m-clipboard",
        "icon-m-developer-mode",
        "icon-m-file-formatted",
        "icon-m-bubble-universal",
        "icon-m-sync",
        "icon-m-link",
        "icon-m-pin"
    ]

    canAccept: (buttonTextField ? buttonTextField.text.trim().length > 0 : false) &&
               (instructionArea ? instructionArea.text.trim().length > 0 : false) &&
               selectedIcon.length > 0

    onAccepted: {
        var id = instructionId.length > 0 ? instructionId : ("custom_" + Date.now())
        var btnText = buttonTextField.text.trim()
        var icon = selectedIcon
        var inst = instructionArea.text.trim()

        if (typeof app !== "undefined" && app.saveCustomAiInstruction) {
            app.saveCustomAiInstruction({
                "id": id,
                "buttonText": btnText,
                "icon": icon,
                "instruction": inst
            })
        }
    }

    SilicaFlickable {
        anchors.fill: parent
        contentHeight: contentColumn.height + Theme.paddingLarge

        Column {
            id: contentColumn
            width: parent.width
            spacing: Theme.paddingMedium

            DialogHeader {
                title: customInstructionDialog.isEdit ? qsTr("Edit AI Instruction") : qsTr("New AI Instruction")
                acceptText: customInstructionDialog.isEdit ? qsTr("Save") : qsTr("Create")
                cancelText: qsTr("Cancel")
            }

            // Button Text Field
            TextField {
                id: buttonTextField
                width: parent.width
                text: customInstructionDialog.effectiveInitialButtonText
                label: qsTr("Button text")
                placeholderText: qsTr("e.g. Summarize, Polish, Translate...")
                focus: !customInstructionDialog.isEdit
                EnterKey.iconSource: "image://theme/icon-m-enter-next"
                EnterKey.onClicked: instructionArea.focus = true
            }

            // Icon Selection Section
            SectionHeader {
                text: qsTr("Button Icon")
            }

            // Selected Icon Preview Card
            Rectangle {
                width: parent.width - Theme.horizontalPageMargin * 2
                height: iconCardRow.height + Theme.paddingMedium * 2
                anchors.horizontalCenter: parent.horizontalCenter
                radius: Theme.paddingSmall
                color: Theme.rgba(Theme.highlightBackgroundColor, 0.15)
                border.color: Theme.rgba(Theme.primaryColor, 0.2)
                border.width: 1

                Row {
                    id: iconCardRow
                    anchors {
                        left: parent.left
                        right: parent.right
                        verticalCenter: parent.verticalCenter
                        margins: Theme.paddingMedium
                    }
                    spacing: Theme.paddingMedium

                    Rectangle {
                        width: Theme.itemSizeExtraSmall
                        height: Theme.itemSizeExtraSmall
                        radius: Theme.paddingSmall / 2
                        color: Theme.rgba(Theme.primaryColor, 0.08)
                        anchors.verticalCenter: parent.verticalCenter

                        Icon {
                            anchors.centerIn: parent
                            source: customInstructionDialog.selectedIcon.length > 0 ? "image://theme/" + customInstructionDialog.selectedIcon : "image://theme/icon-m-note"
                            width: Theme.iconSizeMedium
                            height: Theme.iconSizeMedium
                            color: Theme.primaryColor
                        }
                    }

                    Column {
                        width: parent.width - Theme.itemSizeExtraSmall - pickBtn.width - Theme.paddingMedium * 2
                        anchors.verticalCenter: parent.verticalCenter

                        Label {
                            text: qsTr("Selected icon")
                            font.pixelSize: Theme.fontSizeExtraSmall
                            color: Theme.secondaryColor
                        }

                        Label {
                            text: customInstructionDialog.selectedIcon
                            font.pixelSize: Theme.fontSizeSmall
                            font.bold: true
                            color: Theme.primaryColor
                            truncationMode: TruncationMode.Fade
                            width: parent.width
                        }
                    }

                    Button {
                        id: pickBtn
                        text: qsTr("Choose...")
                        preferredWidth: Theme.buttonWidthExtraSmall
                        anchors.verticalCenter: parent.verticalCenter
                        onClicked: {
                            var dialog = pageStack.push(Qt.resolvedUrl("IconPickerDialog.qml"), {
                                "selectedIcon": customInstructionDialog.selectedIcon
                            })
                            dialog.accepted.connect(function() {
                                customInstructionDialog.selectedIcon = dialog.selectedIcon
                            })
                        }
                    }
                }
            }

            // Popular Icons Quick Strip
            Label {
                x: Theme.horizontalPageMargin
                text: qsTr("Quick suggestions:")
                font.pixelSize: Theme.fontSizeExtraSmall
                color: Theme.secondaryColor
            }

            Flow {
                width: parent.width - Theme.horizontalPageMargin * 2
                anchors.horizontalCenter: parent.horizontalCenter
                spacing: Theme.paddingSmall

                Repeater {
                    model: customInstructionDialog.popularIcons

                    delegate: BackgroundItem {
                        id: popIconItem
                        width: Theme.itemSizeExtraSmall
                        height: Theme.itemSizeExtraSmall
                        highlighted: customInstructionDialog.selectedIcon === modelData

                        Rectangle {
                            anchors.fill: parent
                            radius: Theme.paddingSmall / 2
                            color: popIconItem.highlighted ? Theme.rgba(Theme.highlightBackgroundColor, 0.4) : (popIconItem.down ? Theme.rgba(Theme.primaryColor, 0.1) : Theme.rgba(Theme.primaryColor, 0.05))
                            border.color: popIconItem.highlighted ? Theme.highlightColor : Theme.rgba(Theme.primaryColor, 0.15)
                            border.width: popIconItem.highlighted ? 2 : 1

                            Icon {
                                anchors.centerIn: parent
                                source: "image://theme/" + modelData
                                width: Theme.iconSizeSmall
                                height: Theme.iconSizeSmall
                                color: popIconItem.highlighted ? Theme.highlightColor : Theme.primaryColor
                            }
                        }

                        onClicked: {
                            customInstructionDialog.selectedIcon = modelData
                        }
                    }
                }
            }

            // Instruction Prompt Text Area
            SectionHeader {
                text: qsTr("Instruction Prompt")
            }

            TextArea {
                id: instructionArea
                width: parent.width
                text: customInstructionDialog.effectiveInitialInstruction
                label: qsTr("Instruction prompt")
                placeholderText: qsTr("e.g. Please summarize the key ideas into a concise 3-bullet AsciiDoc list with action items.")
            }

            // Reset to Default button (shown when editing a standard/built-in instruction)
            Button {
                anchors.horizontalCenter: parent.horizontalCenter
                preferredWidth: Theme.buttonWidthMedium
                text: qsTr("Reset to Default Prompt")
                visible: customInstructionDialog.isStandardInstruction
                onClicked: {
                    if (customInstructionDialog.defaultInstructionItem) {
                        buttonTextField.text = customInstructionDialog.defaultInstructionItem.buttonText || ""
                        customInstructionDialog.selectedIcon = customInstructionDialog.defaultInstructionItem.icon || "icon-m-note"
                        instructionArea.text = customInstructionDialog.defaultInstructionItem.instruction || ""
                    }
                }
            }

            // Help card explaining variables
            Rectangle {
                width: parent.width - Theme.horizontalPageMargin * 2
                anchors.horizontalCenter: parent.horizontalCenter
                height: helpCol.height + Theme.paddingMedium * 2
                radius: Theme.paddingSmall
                color: Theme.rgba(Theme.primaryColor, 0.05)
                border.color: Theme.rgba(Theme.primaryColor, 0.15)
                border.width: 1

                Column {
                    id: helpCol
                    anchors {
                        left: parent.left
                        right: parent.right
                        top: parent.top
                        margins: Theme.paddingMedium
                    }
                    spacing: 4

                    Label {
                        text: qsTr("Optional Placeholders:")
                        font.pixelSize: Theme.fontSizeExtraSmall
                        font.bold: true
                        color: Theme.primaryColor
                    }

                    Label {
                        width: parent.width
                        text: qsTr("• {content} — Replaced with the active note content\n• {filename} — Replaced with active note filename\n• {input} — Replaced with text typed into prompt bar\n• {context} — Replaced with note & input context\n\nIf no placeholders are used, the active note and input text are automatically appended to your instruction.")
                        font.pixelSize: Theme.fontSizeTiny
                        color: Theme.secondaryColor
                        wrapMode: Text.Wrap
                    }
                }
            }
        }
    }
}
