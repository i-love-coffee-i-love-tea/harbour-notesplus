import QtQuick 2.6
import Sailfish.Silica 1.0
import harbour.notesplus 1.0
import "../components"

Page {
    id: systemPromptPage
    allowedOrientations: Orientation.All

    AgentBridge {
        id: promptBridge
    }

    readonly property string basePromptText: promptBridge ? promptBridge.defaultSystemPrompt() : ""

    SilicaFlickable {
        anchors.fill: parent
        contentHeight: column.height + Theme.paddingLarge

        PullDownMenu {
            MenuItem {
                text: qsTr("Clear Custom Instructions")
                enabled: customPromptArea.text.trim().length > 0
                onClicked: {
                    customPromptArea.text = ""
                    if (typeof app !== "undefined" && app && app.setAiSystemPrompt) {
                        app.setAiSystemPrompt("")
                    }
                }
            }
        }

        Column {
            id: column
            width: parent.width
            spacing: Theme.paddingMedium

            PageHeader {
                title: qsTr("System Prompt & Persona")
                description: qsTr("Base template & global instructions")
            }

            // Section 1: Custom System Instructions
            SectionHeader {
                text: qsTr("Custom System Instructions")
            }

            Label {
                x: Theme.horizontalPageMargin
                width: parent.width - Theme.horizontalPageMargin * 2
                text: qsTr("Add persistent instructions that augment the assistant for all conversations (e.g., 'Never output JSON in chat messages', 'Always reply in bullet points', or translation preferences). These rules are layered with the base persona.")
                font.pixelSize: Theme.fontSizeExtraSmall
                color: Theme.secondaryColor
                wrapMode: Text.Wrap
            }

            TextArea {
                id: customPromptArea
                width: parent.width
                label: qsTr("Custom System Instructions")
                placeholderText: qsTr("e.g. Always format answers as clean, readable text. Additional user preferences.")
                text: (typeof app !== "undefined" && app && app.aiSystemPrompt) ? app.aiSystemPrompt : ""
                onTextChanged: {
                    if (typeof app !== "undefined" && app && app.setAiSystemPrompt) {
                        app.setAiSystemPrompt(text)
                    }
                }
            }

            // Section 2: Base System Prompt Template
            SectionHeader {
                text: qsTr("Base System Prompt Template")
            }

            Label {
                x: Theme.horizontalPageMargin
                width: parent.width - Theme.horizontalPageMargin * 2
                text: qsTr("The base template establishes the 'Notes Plus Assistant' identity, AsciiDoc domain rules, tone matching, concise bullet formatting, group organization rules, available note manipulation tools (reading, creating, moving across groups, section editing, inserting, appending, context retrieval), and safety guidelines. Custom instructions above augment this base prompt.")
                font.pixelSize: Theme.fontSizeExtraSmall
                color: Theme.secondaryColor
                wrapMode: Text.Wrap
            }

            Rectangle {
                x: Theme.horizontalPageMargin
                width: parent.width - Theme.horizontalPageMargin * 2
                height: basePromptLabel.height + Theme.paddingMedium * 2
                color: Theme.rgba(Theme.highlightBackgroundColor, 0.1)
                radius: Theme.paddingSmall
                border.color: Theme.rgba(Theme.highlightColor, 0.3)
                border.width: 1

                Label {
                    id: basePromptLabel
                    anchors.centerIn: parent
                    width: parent.width - Theme.paddingMedium * 2
                    text: systemPromptPage.basePromptText
                    font.family: Theme.fontFamilyHeading
                    font.pixelSize: Theme.fontSizeExtraSmall - 1
                    color: Theme.primaryColor
                    wrapMode: Text.Wrap
                }
            }

            Item {
                width: parent.width
                height: Theme.paddingLarge
            }
        }
    }
}
