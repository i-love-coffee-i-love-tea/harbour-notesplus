import QtQuick 2.6
import Sailfish.Silica 1.0
import harbour.notesplus 1.0
import "."

Column {
    id: assistantSettingsTab
    width: parent.width
    spacing: Theme.paddingMedium

    // Properties passed from SettingsPage
    property var page: null        // reference to SettingsPage for shared state
    property var modelBridge: null // AgentBridge instance for model fetching

    SectionHeader {
        text: qsTr("Provider")
    }

    TextSwitch {
        width: parent.width
        text: qsTr("Enable AI Features")
        description: qsTr("Enable AI Assistant, action templates, and note import tools")
        checked: (typeof app !== "undefined" && app && app.aiEnabled !== undefined) ? app.aiEnabled : true
        onCheckedChanged: {
            if (typeof app !== "undefined" && app && app.setAiEnabled) {
                app.setAiEnabled(checked)
            }
        }
    }

    BackgroundItem {
        width: parent.width
        onClicked: pageStack.push(Qt.resolvedUrl("../../pages/AiCapabilitiesPage.qml"))

        Label {
            x: Theme.horizontalPageMargin
            width: parent.width - Theme.horizontalPageMargin * 2
            text: qsTr("What can the AI do with my notes?")
            color: parent.highlighted ? Theme.highlightColor : Theme.primaryColor
            font.pixelSize: Theme.fontSizeSmall
            font.underline: true
            wrapMode: Text.Wrap
        }
    }

    Column {
        width: parent.width
        spacing: Theme.paddingMedium
        visible: (typeof app !== "undefined" && app && app.aiEnabled !== undefined) ? app.aiEnabled : true

        ComboBox {
            width: parent.width
            label: qsTr("AI Provider")
            currentIndex: (app.aiProvider === "mimocode" || app.aiProvider === "openai") ? 1 : 0
            menu: ContextMenu {
                MenuItem {
                    text: qsTr("Ollama")
                    onClicked: {
                        app.setAiProvider("ollama")
                        if (endpointField.text === "https://api.mimocode.com") {
                            endpointField.text = app.defaultAiEndpoint
                            app.setAiEndpoint(app.defaultAiEndpoint)
                        }
                        page.modelsLoaded = false
                        page.refreshModels()
                    }
                }
                MenuItem {
                    text: qsTr("Xiaomi MiMoCode / OpenAI Compatible API")
                    onClicked: {
                        app.setAiProvider("mimocode")
                        if (endpointField.text === app.defaultAiEndpoint) {
                            endpointField.text = "https://api.mimocode.com"
                            app.setAiEndpoint("https://api.mimocode.com")
                        }
                        page.modelsLoaded = false
                        page.refreshModels()
                    }
                }
            }
        }

        TextField {
            id: endpointField
            width: parent.width
            label: qsTr("Endpoint URL")
            labelVisible: true
            placeholderText: app.aiProvider === "ollama" ? app.defaultAiEndpoint : "https://api.mimocode.com"
            text: app.aiEndpoint
            onTextChanged: {
                if (typeof app !== "undefined" && app && app.setAiEndpoint) {
                    app.setAiEndpoint(text)
                }
            }
        }

        TextSwitch {
            width: parent.width
            text: qsTr("Accept Self-Signed Certificates")
            description: qsTr("Allow connecting to Ollama endpoints or proxies using self-signed or internal SSL certificates")
            checked: (typeof app !== "undefined" && app && app.aiAllowSelfSigned !== undefined) ? app.aiAllowSelfSigned : false
            onCheckedChanged: {
                if (typeof app !== "undefined" && app && app.setAiAllowSelfSigned) {
                    app.setAiAllowSelfSigned(checked)
                }
            }
        }

        PasswordField {
            id: apiKeyField
            width: parent.width
            label: qsTr("API Key (Bearer Token)")
            labelVisible: true
            placeholderText: app.aiProvider === "ollama" ? qsTr("API Key (optional for Ollama)") : qsTr("API Key (required for MiMoCode / Cloud APIs)")
            text: app.aiApiKey
            onTextChanged: {
                if (typeof app !== "undefined" && app && app.setAiApiKey) {
                    app.setAiApiKey(text)
                }
            }
        }

        Slider {
            width: parent.width
            minimumValue: 15
            maximumValue: 1200
            stepSize: 15
            value: (typeof app !== "undefined" && app && app.aiTimeout !== undefined) ? app.aiTimeout : 90
            label: qsTr("Request Timeout")
            valueText: qsTr("%1 seconds").arg(Math.round(value))
            onSliderValueChanged: {
                if (typeof app !== "undefined" && app && app.setAiTimeout) {
                    app.setAiTimeout(Math.round(value))
                }
            }
        }
    }

    SectionHeader {
        text: qsTr("Model")
    }

    Column {
        width: parent.width
        spacing: Theme.paddingMedium
        visible: (typeof app !== "undefined" && app && app.aiEnabled !== undefined) ? app.aiEnabled : true

        ComboBox {
            id: modelCombo
            width: parent.width
            label: qsTr("Model")
            property bool userSelecting: false
            currentIndex: {
                var models = page.displayModels
                var current = (typeof app !== "undefined" && app.aiModel) ? app.aiModel : ""
                for (var i = 0; i < models.length; i++) {
                    if ((models[i].id || models[i].name) === current) return i
                }
                return models.length > 0 ? 0 : -1
            }
            menu: ContextMenu {
                Repeater {
                    model: page.displayModels
                    MenuItem {
                        text: modelData.name || modelData.id || ""
                        onClicked: modelCombo.userSelecting = true
                    }
                }
            }
            onCurrentIndexChanged: {
                if (!userSelecting) return
                userSelecting = false
                var models = page.displayModels
                if (currentIndex >= 0 && currentIndex < models.length) {
                    var selectedId = models[currentIndex].id || models[currentIndex].name
                    if (typeof app !== "undefined" && app && app.setAiModel && selectedId !== app.aiModel) {
                        app.setAiModel(selectedId)
                    }
                }
            }
            description: {
                if (modelBridge.models_loading) return qsTr("Fetching models from server...")
                if (page.displayModels.length === 0) return qsTr("No models found — check endpoint")
                return ""
            }
        }

        Label {
            visible: page.modelError && page.modelError.length > 0
            x: Theme.horizontalPageMargin
            width: parent.width - Theme.horizontalPageMargin * 2
            text: page.modelError
            color: Theme.errorColor
            font.pixelSize: Theme.fontSizeSmall
            wrapMode: Text.Wrap
        }

        Row {
            anchors.horizontalCenter: parent.horizontalCenter
            spacing: Theme.paddingMedium

            Button {
                text: qsTr("Refresh Models")
                enabled: !modelBridge.models_loading
                onClicked: page.refreshModels()
            }
        }
    }

    SectionHeader {
        text: qsTr("Tool Permissions")
    }

    Column {
        width: parent.width
        spacing: Theme.paddingMedium
        visible: (typeof app !== "undefined" && app && app.aiEnabled !== undefined) ? app.aiEnabled : true

        TextSwitch {
            width: parent.width
            text: qsTr("Auto-Allow Note Reading & Context Retrieval")
            description: qsTr("Allow the assistant to search notes, read note contents, list groups, and perform local SQLite FTS5 hybrid context retrieval automatically (read_note, list_notes, search_notes, retrieve_context, list_groups)")
            checked: app.aiAutoAllowRead
            onCheckedChanged: {
                if (typeof app !== "undefined" && app && app.setAiAutoAllowRead) {
                    app.setAiAutoAllowRead(checked)
                }
            }
        }

        TextSwitch {
            width: parent.width
            text: qsTr("Auto-Allow Note Creation")
            description: qsTr("Allow the assistant to create new AsciiDoc notes without confirmation (create_note)")
            checked: app.aiAutoAllowCreate
            onCheckedChanged: {
                if (typeof app !== "undefined" && app && app.setAiAutoAllowCreate) {
                    app.setAiAutoAllowCreate(checked)
                }
            }
        }

        TextSwitch {
            width: parent.width
            text: qsTr("Require Confirmation for Note Modifications")
            description: qsTr("Display confirmation and preview before modifying notes, editing sections, or moving notes between groups (edit_note, edit_section, append_to_note, insert_section, move_note)")
            checked: app.aiRequireConfirmEdit
            onCheckedChanged: {
                if (typeof app !== "undefined" && app && app.setAiRequireConfirmEdit) {
                    app.setAiRequireConfirmEdit(checked)
                }
            }
        }

        TextSwitch {
            width: parent.width
            text: qsTr("Allow Web Requests")
            description: qsTr("Allow the assistant to fetch and analyze content from URLs you mention in messages (fetch_url)")
            checked: app.aiAllowFetchUrl
            onCheckedChanged: {
                if (typeof app !== "undefined" && app && app.setAiAllowFetchUrl) {
                    app.setAiAllowFetchUrl(checked)
                }
            }
        }
    }

    SectionHeader {
        text: qsTr("System Prompt & Persona")
    }

    Column {
        width: parent.width
        spacing: Theme.paddingMedium
        visible: (typeof app !== "undefined" && app && app.aiEnabled !== undefined) ? app.aiEnabled : true

        Label {
            width: parent.width - Theme.horizontalPageMargin * 2
            anchors.horizontalCenter: parent.horizontalCenter
            text: qsTr("Inspect the base system prompt template (AsciiDoc syntax rules, persona, group organization, available tools) and add custom instructions (e.g., specific format or translation preferences) that apply across all chat turns.")
            font.pixelSize: Theme.fontSizeExtraSmall
            color: Theme.secondaryColor
            wrapMode: Text.Wrap
        }

        Row {
            anchors.horizontalCenter: parent.horizontalCenter
            spacing: Theme.paddingMedium

            Button {
                text: ((typeof app !== "undefined" && app && app.aiSystemPrompt && app.aiSystemPrompt.trim().length > 0)
                      ? qsTr("View & Edit System Prompt (Customized)")
                      : qsTr("View & Edit System Prompt..."))
                onClicked: pageStack.push(Qt.resolvedUrl("../../pages/SystemPromptPage.qml"))
            }
        }
    }

    SectionHeader {
        text: qsTr("Quick Action Templates")
    }

    Column {
        width: parent.width
        spacing: Theme.paddingMedium
        visible: (typeof app !== "undefined" && app && app.aiEnabled !== undefined) ? app.aiEnabled : true

        Label {
            width: parent.width - Theme.horizontalPageMargin * 2
            anchors.horizontalCenter: parent.horizontalCenter
            text: qsTr("Inspect and customize one-tap action templates (Beautify, Extract To-Dos, Fix Grammar, Expand & Draft) and custom prompt buttons.")
            font.pixelSize: Theme.fontSizeExtraSmall
            color: Theme.secondaryColor
            wrapMode: Text.Wrap
        }

        Row {
            anchors.horizontalCenter: parent.horizontalCenter
            spacing: Theme.paddingMedium

            Button {
                text: qsTr("Manage Templates (%1)").arg((typeof app !== "undefined" && app.customAiInstructions) ? app.customAiInstructions.length : 0)
                onClicked: pageStack.push(Qt.resolvedUrl("../../pages/CustomInstructionsPage.qml"))
            }

            Button {
                text: qsTr("+ Add New")
                onClicked: pageStack.push(Qt.resolvedUrl("../../dialogs/CustomInstructionDialog.qml"))
            }
        }
    }
}
