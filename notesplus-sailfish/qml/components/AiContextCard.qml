import QtQuick 2.6
import Sailfish.Silica 1.0

Rectangle {
    id: contextCard
    width: parent.width - Theme.horizontalPageMargin * 2
    anchors.horizontalCenter: parent.horizontalCenter
    height: contentColumn.height + Theme.paddingMedium * 2
    radius: Theme.paddingSmall

    property string contextFilename: ""
    property string contextContent: ""
    property string extraContext: ""
    property bool agentBusy: false

    readonly property bool hasContext: (contextFilename.length > 0) || (contextContent.length > 0) || (extraContext.length > 0)

    color: hasContext ? Theme.rgba(Theme.highlightBackgroundColor, 0.15) : Theme.rgba(Theme.primaryColor, 0.05)
    border.color: hasContext ? Theme.rgba(Theme.primaryColor, 0.25) : Theme.rgba(Theme.primaryColor, 0.18)
    border.width: 1

    signal attachNoteRequested()
    signal detachNoteRequested()
    signal attachClipboardRequested()
    signal detachExtraContextRequested()
    signal clearAllContextRequested()

    Column {
        id: contentColumn
        anchors {
            left: parent.left
            right: parent.right
            top: parent.top
            margins: Theme.paddingMedium
        }
        spacing: Theme.paddingSmall

        // Header with Title and optional Clear All Action
        Item {
            width: parent.width
            height: Theme.itemSizeExtraSmall

            Row {
                anchors.left: parent.left
                anchors.verticalCenter: parent.verticalCenter
                spacing: Theme.paddingSmall

                Icon {
                    source: contextCard.hasContext ? "image://theme/icon-m-attach" : "image://theme/icon-m-about"
                    width: Theme.iconSizeSmall
                    height: Theme.iconSizeSmall
                    color: contextCard.hasContext ? Theme.primaryColor : Theme.secondaryColor
                    anchors.verticalCenter: parent.verticalCenter
                }

                Label {
                    text: contextCard.hasContext ? qsTr("Attached Context") : qsTr("Context")
                    font.bold: true
                    font.pixelSize: Theme.fontSizeSmall
                    color: contextCard.hasContext ? Theme.primaryColor : Theme.secondaryColor
                    anchors.verticalCenter: parent.verticalCenter
                }
            }

            IconButton {
                id: clearAllBtn
                icon.source: "image://theme/icon-m-clear"
                icon.width: Theme.iconSizeSmall
                icon.height: Theme.iconSizeSmall
                width: Theme.itemSizeExtraSmall
                height: Theme.itemSizeExtraSmall
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                visible: contextCard.hasContext
                enabled: !contextCard.agentBusy
                onClicked: contextCard.clearAllContextRequested()
            }
        }

        // Active Context Items (if attached)
        Column {
            width: parent.width
            spacing: Theme.paddingSmall
            visible: contextCard.hasContext

            // Attached Note File
            Rectangle {
                width: parent.width
                height: noteRow.height + Theme.paddingSmall * 2
                visible: contextCard.contextFilename.length > 0 || contextCard.contextContent.length > 0
                color: Theme.rgba(Theme.primaryColor, 0.06)
                radius: Theme.paddingSmall / 2
                border.color: Theme.rgba(Theme.primaryColor, 0.2)
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
                        color: Theme.primaryColor
                        anchors.verticalCenter: parent.verticalCenter
                    }

                    Column {
                        width: parent.width - Theme.iconSizeSmall - detachNoteBtn.width - Theme.paddingSmall * 2
                        anchors.verticalCenter: parent.verticalCenter

                        Label {
                            text: contextCard.contextFilename.length > 0 ? contextCard.contextFilename : qsTr("Active Note")
                            font.pixelSize: Theme.fontSizeExtraSmall
                            font.bold: true
                            color: Theme.primaryColor
                            truncationMode: TruncationMode.Fade
                            width: parent.width
                        }

                        Label {
                            text: {
                                var chars = contextCard.contextContent.length
                                var lines = contextCard.contextContent.length > 0 ? contextCard.contextContent.split("\n").length : 0
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
                        enabled: !contextCard.agentBusy
                        anchors.verticalCenter: parent.verticalCenter
                        onClicked: contextCard.detachNoteRequested()
                    }
                }
            }

            // Attached Extra Context / Clipboard / Snippet
            Rectangle {
                width: parent.width
                height: extraRow.height + Theme.paddingSmall * 2
                visible: contextCard.extraContext.length > 0
                color: Theme.rgba(Theme.primaryColor, 0.06)
                radius: Theme.paddingSmall / 2
                border.color: Theme.rgba(Theme.primaryColor, 0.2)
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
                        color: Theme.primaryColor
                        anchors.verticalCenter: parent.verticalCenter
                    }

                    Column {
                        width: parent.width - Theme.iconSizeSmall - detachExtraBtn.width - Theme.paddingSmall * 2
                        anchors.verticalCenter: parent.verticalCenter

                        Label {
                            text: qsTr("Clipboard / Extra Context")
                            font.pixelSize: Theme.fontSizeExtraSmall
                            font.bold: true
                            color: Theme.primaryColor
                            truncationMode: TruncationMode.Fade
                            width: parent.width
                        }

                        Label {
                            text: {
                                var chars = contextCard.extraContext.length
                                var lines = contextCard.extraContext.length > 0 ? contextCard.extraContext.split("\n").length : 0
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
                        enabled: !contextCard.agentBusy
                        anchors.verticalCenter: parent.verticalCenter
                        onClicked: contextCard.detachExtraContextRequested()
                    }
                }
            }
        }

        // Description when NO context data is attached
        Label {
            width: parent.width
            visible: !contextCard.hasContext
            text: qsTr("No note or clipboard context attached. The AI will answer using general knowledge and available note tools.")
            font.pixelSize: Theme.fontSizeExtraSmall
            color: Theme.secondaryColor
            wrapMode: Text.Wrap
        }

        // Action Buttons Row (When no context attached)
        Row {
            anchors.horizontalCenter: parent.horizontalCenter
            spacing: Theme.paddingMedium
            visible: !contextCard.hasContext

            Button {
                text: qsTr("Attach Note")
                preferredWidth: Theme.buttonWidthSmall
                enabled: !contextCard.agentBusy
                onClicked: contextCard.attachNoteRequested()
            }

            Button {
                text: qsTr("Attach Clipboard")
                preferredWidth: Theme.buttonWidthSmall
                enabled: !contextCard.agentBusy
                onClicked: contextCard.attachClipboardRequested()
            }
        }

        // Secondary Action Buttons Row (When partial context is attached)
        Row {
            anchors.horizontalCenter: parent.horizontalCenter
            spacing: Theme.paddingMedium
            visible: contextCard.hasContext && ((contextCard.contextFilename.length === 0 && contextCard.contextContent.length === 0) || contextCard.extraContext.length === 0)

            Button {
                text: qsTr("Attach Note")
                preferredWidth: Theme.buttonWidthSmall
                visible: contextCard.contextFilename.length === 0 && contextCard.contextContent.length === 0
                enabled: !contextCard.agentBusy
                onClicked: contextCard.attachNoteRequested()
            }

            Button {
                text: qsTr("Attach Clipboard")
                preferredWidth: Theme.buttonWidthSmall
                visible: contextCard.extraContext.length === 0
                enabled: !contextCard.agentBusy
                onClicked: contextCard.attachClipboardRequested()
            }
        }
    }
}
