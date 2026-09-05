import QtQuick 2.0
import Sailfish.Silica 1.0
import harbour.fishdoc 1.0
import "../components"

Page {
    id: assistantPage

    property string contextFilename: ""
    property string contextContent: ""

    function applyConfig() {
        if (typeof agentBridge !== "undefined" && agentBridge) {
            agentBridge.configure(
                app.aiProvider || "ollama",
                app.aiEndpoint || "http://192.168.1.1:11434",
                app.aiModel || "llama3.2",
                app.aiApiKey || "",
                app.aiTimeout || 90,
                app.aiAutoAllowRead !== undefined ? app.aiAutoAllowRead : true,
                app.aiAutoAllowCreate !== undefined ? app.aiAutoAllowCreate : true,
                app.aiRequireConfirmEdit !== undefined ? app.aiRequireConfirmEdit : true
            )
        }
    }

    onStatusChanged: {
        if (status === PageStatus.Active) {
            applyConfig()
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

        onUndo_completed: {
            remorsePopup.execute(message, function() {})
        }
    }

    Timer {
        id: pollTimer
        interval: 120
        running: agentBridge.agent_busy
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
                text: "Settings"
                onClicked: pageStack.push(Qt.resolvedUrl("SettingsPage.qml"))
            }
            MenuItem {
                text: "Paste Clipboard Context"
                onClicked: {
                    if (Clipboard.text && Clipboard.text.length > 0) {
                        agentBridge.reset_session(contextFilename, contextContent, Clipboard.text)
                        promptField.text = "Please analyze the clipboard content."
                    }
                }
            }
            MenuItem {
                text: "Clear Conversation"
                onClicked: {
                    agentBridge.reset_session(contextFilename, contextContent, "")
                }
            }
        }

        Column {
            id: mainColumn
            width: parent.width
            spacing: Theme.paddingMedium

            PageHeader {
                title: "AI Assistant"
                description: contextFilename.length > 0 ? ("Context: " + contextFilename) : ""
            }

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
                        text: "✨ Beautify"
                        preferredWidth: Theme.buttonWidthExtraSmall
                        enabled: !agentBridge.agent_busy
                        onClicked: {
                            assistantPage.applyConfig()
                            agentBridge.run_template("beautify", promptField.text, contextContent)
                            promptField.text = ""
                        }
                    }

                    Button {
                        text: "📋 Extract To-Dos"
                        preferredWidth: Theme.buttonWidthExtraSmall
                        enabled: !agentBridge.agent_busy
                        onClicked: {
                            assistantPage.applyConfig()
                            agentBridge.run_template("extract_todos", promptField.text, contextContent)
                            promptField.text = ""
                        }
                    }

                    Button {
                        text: "✍️ Fix Grammar"
                        preferredWidth: Theme.buttonWidthExtraSmall
                        enabled: !agentBridge.agent_busy
                        onClicked: {
                            assistantPage.applyConfig()
                            agentBridge.run_template("fix_grammar", promptField.text, contextContent)
                            promptField.text = ""
                        }
                    }

                    Button {
                        text: "📝 Expand & Draft"
                        preferredWidth: Theme.buttonWidthExtraSmall
                        enabled: !agentBridge.agent_busy
                        onClicked: {
                            assistantPage.applyConfig()
                            agentBridge.run_template("expand_draft", promptField.text, contextContent)
                            promptField.text = ""
                        }
                    }

                    Button {
                        text: "🌐 External Text"
                        preferredWidth: Theme.buttonWidthExtraSmall
                        enabled: !agentBridge.agent_busy
                        onClicked: {
                            assistantPage.applyConfig()
                            agentBridge.run_template("analyze_external", promptField.text, contextContent)
                            promptField.text = ""
                        }
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
                                            if (modelData.role === "user") return "You"
                                            if (modelData.role === "tool") return "🔧 Tool Output"
                                            return "🤖 Assistant"
                                        }
                                        font.pixelSize: Theme.fontSizeExtraSmall
                                        font.bold: true
                                        color: Theme.highlightColor
                                    }
                                }

                                Label {
                                    width: parent.width
                                    text: modelData.content || (modelData.tool_calls ? "Running note tools..." : "")
                                    font.pixelSize: Theme.fontSizeSmall
                                    color: Theme.primaryColor
                                    wrapMode: Text.Wrap
                                }
                            }
                        }
                    }
                }
            }

            // Busy Indicator
            Item {
                width: parent.width
                height: Theme.itemSizeMedium
                visible: agentBridge.agent_busy

                BusyIndicator {
                    anchors.centerIn: parent
                    running: agentBridge.agent_busy
                    size: BusyIndicatorSize.Small
                }
            }

            // Input Area
            Row {
                width: parent.width - Theme.horizontalPageMargin * 2
                anchors.horizontalCenter: parent.horizontalCenter
                spacing: Theme.paddingSmall

                TextField {
                    id: promptField
                    width: parent.width - sendBtn.width - Theme.paddingSmall
                    placeholderText: "Ask assistant or run template..."
                    label: "Prompt"
                    enabled: !agentBridge.agent_busy
                    EnterKey.enabled: text.length > 0
                    EnterKey.iconSource: "image://theme/icon-m-send"
                    EnterKey.onClicked: {
                        if (text.length > 0) {
                            assistantPage.applyConfig()
                            agentBridge.send_prompt(text)
                            text = ""
                        }
                    }
                }

                IconButton {
                    id: sendBtn
                    icon.source: "image://theme/icon-m-send"
                    enabled: !agentBridge.agent_busy && promptField.text.length > 0
                    anchors.verticalCenter: promptField.verticalCenter
                    onClicked: {
                        if (promptField.text.length > 0) {
                            assistantPage.applyConfig()
                            agentBridge.send_prompt(promptField.text)
                            promptField.text = ""
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
