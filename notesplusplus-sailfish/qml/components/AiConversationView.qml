import QtQuick 2.0
import Sailfish.Silica 1.0

Column {
    id: conversationView
    width: parent.width - Theme.horizontalPageMargin * 2
    anchors.horizontalCenter: parent.horizontalCenter
    spacing: Theme.paddingMedium

    property string messagesJson: ""
    property bool agentBusy: false
    property string streamingText: ""

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
        visible: conversationView.agentBusy && conversationView.streamingText.length > 0

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
                    text: conversationView.streamingText
                    font.pixelSize: Theme.fontSizeSmall
                    color: Theme.primaryColor
                    wrapMode: Text.Wrap
                }
            }
        }
    }

    // Busy Indicator (when waiting for first token or executing tools)
    Item {
        width: parent.width
        height: Theme.itemSizeMedium
        visible: conversationView.agentBusy && conversationView.streamingText.length === 0

        BusyIndicator {
            anchors.centerIn: parent
            running: conversationView.agentBusy && conversationView.streamingText.length === 0
            size: BusyIndicatorSize.Small
        }
    }
}
