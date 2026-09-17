import QtQuick 2.6
import Sailfish.Silica 1.0

Column {
    id: conversationView
    width: parent.width - Theme.horizontalPageMargin * 2
    anchors.horizontalCenter: parent.horizontalCenter
    spacing: Theme.paddingMedium

    property string messagesJson: ""
    property bool agentBusy: false
    property string streamingText: ""

    signal resendRequested(string text)

    function getToolDisplayName(name) {
        if (!name) return qsTr("Tool")
        if (name === "read_note") return qsTr("Read Note")
        if (name === "search_notes") return qsTr("Search Notes")
        if (name === "list_notes") return qsTr("List Notes")
        if (name === "create_note") return qsTr("Create Note")
        if (name === "edit_note") return qsTr("Edit Note")
        if (name === "fetch_url") return qsTr("Fetch Web Page")
        return name
    }

    function getToolIcon(name) {
        if (!name) return "🔧"
        if (name === "read_note") return "📖"
        if (name === "search_notes") return "🔍"
        if (name === "list_notes") return "📋"
        if (name === "create_note") return "📝"
        if (name === "edit_note") return "✏️"
        if (name === "fetch_url") return "🌐"
        return "🔧"
    }

    function formatToolArguments(args) {
        if (!args) return ""
        var obj = args
        if (typeof args === "string") {
            try {
                obj = JSON.parse(args)
            } catch (e) {
                return args
            }
        }
        var lines = []
        for (var key in obj) {
            if (obj.hasOwnProperty(key)) {
                var val = obj[key]
                if (typeof val === "string") {
                    if (val.length > 120) {
                        lines.push(key + ": \"" + val.substring(0, 117) + "...\" (" + val.length + " chars)")
                    } else {
                        lines.push(key + ": \"" + val + "\"")
                    }
                } else if (typeof val === "object") {
                    lines.push(key + ": " + JSON.stringify(val))
                } else {
                    lines.push(key + ": " + val)
                }
            }
        }
        return lines.join("\n")
    }

    Repeater {
        model: {
            try {
                var list = JSON.parse(conversationView.messagesJson)
                // Filter out internal system prompt
                return list.filter(function(m) { return m.role !== "system" })
            } catch (e) {
                return []
            }
        }

        delegate: ListItem {
            id: msgItem
            width: parent.width
            contentHeight: msgBubble.height + Theme.paddingSmall

            menu: isUserMessage ? contextMenuComponent : undefined

            property bool isToolMessage: modelData.role === "tool"
            property bool isAssistantMessage: modelData.role === "assistant"
            property bool isUserMessage: modelData.role === "user"
            property var toolCalls: (modelData.tool_calls && Array.isArray(modelData.tool_calls)) ? modelData.tool_calls : []
            property bool hasToolCalls: toolCalls.length > 0
            property bool hasContent: modelData.content && modelData.content.length > 0
            property bool isExpanded: false

            Component {
                id: contextMenuComponent
                ContextMenu {
                    MenuItem {
                        text: qsTr("Resend")
                        onClicked: {
                            conversationView.resendRequested(modelData.content || "")
                        }
                    }
                }
            }

            Rectangle {
                id: msgBubble
                width: parent.width
                height: msgTextCol.height + Theme.paddingMedium * 2
                radius: Theme.paddingSmall
                color: {
                    if (msgItem.isUserMessage) {
                        return Theme.rgba(Theme.highlightBackgroundColor, 0.4)
                    } else if (msgItem.isToolMessage) {
                        return Theme.rgba(Theme.primaryColor, 0.07)
                    } else if (msgItem.hasToolCalls && !msgItem.hasContent) {
                        return Theme.rgba(Theme.highlightBackgroundColor, 0.12)
                    } else {
                        return Theme.rgba(Theme.highlightBackgroundColor, 0.18)
                    }
                }
                border.color: {
                    if (msgItem.isUserMessage) {
                        return Theme.rgba(Theme.highlightColor, 0.35)
                    } else if (msgItem.isToolMessage) {
                        return Theme.rgba(Theme.secondaryColor, 0.25)
                    } else {
                        return Theme.rgba(Theme.highlightColor, 0.25)
                    }
                }
                border.width: 1

                Column {
                    id: msgTextCol
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: parent.top
                    anchors.margins: Theme.paddingMedium
                    spacing: Theme.paddingSmall

                    // Message Header Row
                    Row {
                        width: parent.width
                        spacing: Theme.paddingSmall

                        Label {
                            text: {
                                if (msgItem.isUserMessage) {
                                    return qsTr("👤 You")
                                }
                                if (msgItem.isToolMessage) {
                                    var toolName = modelData.name || "tool"
                                    return conversationView.getToolIcon(toolName) + " " + qsTr("Tool Output: %1").arg(conversationView.getToolDisplayName(toolName))
                                }
                                if (msgItem.hasToolCalls) {
                                    var firstTool = msgItem.toolCalls[0].function ? msgItem.toolCalls[0].function.name : ""
                                    return "🤖 " + qsTr("Assistant") + " (" + conversationView.getToolDisplayName(firstTool) + ")"
                                }
                                return "🤖 " + qsTr("Assistant")
                            }
                            font.pixelSize: Theme.fontSizeExtraSmall
                            font.bold: true
                            color: msgItem.isToolMessage ? Theme.secondaryColor : Theme.primaryColor
                            truncationMode: TruncationMode.Fade
                            width: parent.width
                        }
                    }

                    // Text Content (Assistant / User)
                    Label {
                        width: parent.width
                        visible: msgItem.hasContent && !msgItem.isToolMessage
                        text: modelData.content || ""
                        font.pixelSize: Theme.fontSizeSmall
                        color: Theme.primaryColor
                        wrapMode: Text.Wrap
                    }

                    // Tool Invocations Section (when Assistant calls one or more tools)
                    Column {
                        width: parent.width
                        spacing: Theme.paddingSmall
                        visible: msgItem.hasToolCalls

                        Repeater {
                            model: msgItem.toolCalls

                            delegate: Rectangle {
                                width: parent.width
                                height: toolCallInnerCol.height + Theme.paddingSmall * 2
                                radius: Theme.paddingSmall / 2
                                color: Theme.rgba(Theme.primaryColor, 0.06)
                                border.color: Theme.rgba(Theme.primaryColor, 0.2)
                                border.width: 1

                                Column {
                                    id: toolCallInnerCol
                                    anchors {
                                        left: parent.left
                                        right: parent.right
                                        top: parent.top
                                        margins: Theme.paddingSmall
                                    }
                                    spacing: 4

                                    Row {
                                        width: parent.width
                                        spacing: Theme.paddingSmall

                                        Label {
                                            text: {
                                                var fnName = modelData.function ? modelData.function.name : ""
                                                return conversationView.getToolIcon(fnName) + " " + qsTr("Calling: %1").arg(conversationView.getToolDisplayName(fnName))
                                            }
                                            font.pixelSize: Theme.fontSizeExtraSmall
                                            font.bold: true
                                            color: Theme.primaryColor
                                            width: parent.width - badgeLabel.width - Theme.paddingSmall
                                            truncationMode: TruncationMode.Fade
                                        }

                                        Label {
                                            id: badgeLabel
                                            text: qsTr("Tool Action")
                                            font.pixelSize: Theme.fontSizeTiny
                                            color: Theme.secondaryColor
                                        }
                                    }

                                    Label {
                                        width: parent.width
                                        text: conversationView.formatToolArguments(modelData.function ? modelData.function.arguments : null)
                                        font.pixelSize: Theme.fontSizeTiny
                                        font.family: "Monospace"
                                        color: Theme.primaryColor
                                        wrapMode: Text.Wrap
                                        visible: text.length > 0
                                    }
                                }
                            }
                        }
                    }

                    // Tool Result Output Section (for Tool role)
                    Column {
                        width: parent.width
                        spacing: Theme.paddingSmall
                        visible: msgItem.isToolMessage

                        property string rawContent: modelData.content || ""
                        property var lines: rawContent.split("\n")
                        property bool isLong: lines.length > 4 || rawContent.length > 250

                        Label {
                            width: parent.width
                            text: {
                                if (!parent.isLong || msgItem.isExpanded) {
                                    return parent.rawContent
                                } else {
                                    return parent.lines.slice(0, 4).join("\n") + (parent.lines.length > 4 ? "\n..." : "")
                                }
                            }
                            font.pixelSize: Theme.fontSizeExtraSmall
                            font.family: "Monospace"
                            color: Theme.primaryColor
                            wrapMode: Text.Wrap
                        }

                        // Expand / Collapse Toggle Button for long tool results
                        BackgroundItem {
                            width: parent.width
                            height: Theme.itemSizeExtraSmall
                            visible: parent.isLong
                            onClicked: {
                                msgItem.isExpanded = !msgItem.isExpanded
                            }

                            Row {
                                anchors.centerIn: parent
                                spacing: Theme.paddingSmall

                                Icon {
                                    source: msgItem.isExpanded ? "image://theme/icon-m-up" : "image://theme/icon-m-down"
                                    width: Theme.iconSizeSmall
                                    height: Theme.iconSizeSmall
                                    color: Theme.secondaryColor
                                    anchors.verticalCenter: parent.verticalCenter
                                }

                                Label {
                                    text: msgItem.isExpanded ? qsTr("Hide details") : qsTr("Show full output (%1 lines)").arg(parent.parent.lines.length)
                                    font.pixelSize: Theme.fontSizeExtraSmall
                                    color: Theme.secondaryColor
                                    anchors.verticalCenter: parent.verticalCenter
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Live Streaming Assistant Bubble
    Item {
        width: parent.width
        height: liveMsgBubble.height + Theme.paddingSmall
        visible: conversationView.agentBusy && conversationView.streamingText.length > 0

        Rectangle {
            id: liveMsgBubble
            width: parent.width
            height: liveMsgTextCol.height + Theme.paddingMedium * 2
            radius: Theme.paddingSmall
            color: Theme.rgba(Theme.highlightBackgroundColor, 0.18)
            border.color: Theme.rgba(Theme.highlightColor, 0.3)
            border.width: 1

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
                        text: "🤖 " + qsTr("Assistant")
                        font.pixelSize: Theme.fontSizeExtraSmall
                        font.bold: true
                        color: Theme.primaryColor
                    }
                    BusyIndicator {
                        size: BusyIndicatorSize.ExtraSmall
                        running: true
                        anchors.verticalCenter: parent.verticalCenter
                    }
                }

                Label {
                    width: parent.width
                    text: conversationView.streamingText
                    font.pixelSize: Theme.fontSizeSmall
                    color: Theme.primaryColor
                    wrapMode: Text.Wrap
                }
            }
        }
    }

    // Busy Indicator (when waiting for LLM or executing tools)
    Item {
        width: parent.width
        height: Theme.itemSizeMedium
        visible: conversationView.agentBusy && conversationView.streamingText.length === 0

        Row {
            anchors.centerIn: parent
            spacing: Theme.paddingMedium

            BusyIndicator {
                running: conversationView.agentBusy && conversationView.streamingText.length === 0
                size: BusyIndicatorSize.Small
                anchors.verticalCenter: parent.verticalCenter
            }

            Label {
                text: qsTr("AI is thinking & executing tools...")
                font.pixelSize: Theme.fontSizeSmall
                color: Theme.secondaryHighlightColor
                anchors.verticalCenter: parent.verticalCenter
            }
        }
    }
}
