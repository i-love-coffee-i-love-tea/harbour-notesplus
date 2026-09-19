import QtQuick 2.6
import Sailfish.Silica 1.0

Rectangle {
    id: confirmationCardRoot
    width: parent.width
    height: cardContent.height + Theme.paddingMedium * 2
    color: Theme.rgba(Theme.highlightBackgroundColor, 0.25)
    border.color: Theme.rgba(Theme.primaryColor, 0.3)
    border.width: 1
    radius: Theme.paddingSmall
    clip: true

    property var actionData: null
    signal confirmed(bool approved)

    Column {
        id: cardContent
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.margins: Theme.paddingMedium
        spacing: Theme.paddingSmall

        Row {
            width: parent.width
            spacing: Theme.paddingSmall

            Icon {
                source: (actionData && actionData.tool_name === "move_note")
                        ? "image://theme/icon-m-folder"
                        : "image://theme/icon-m-edit"
                anchors.verticalCenter: parent.verticalCenter
            }

            Label {
                text: actionData ? ((actionData.tool_name === "move_note" ? qsTr("Move Note: ") : qsTr("Proposed Edit: ")) + (actionData.filename || "")) : qsTr("Action Confirmation")
                font.bold: true
                font.pixelSize: Theme.fontSizeMedium
                color: Theme.primaryColor
                anchors.verticalCenter: parent.verticalCenter
                truncationMode: TruncationMode.Fade
                width: parent.width - Theme.itemSizeExtraSmall
            }
        }

        Label {
            width: parent.width
            text: actionData ? (actionData.reason || "") : ""
            font.pixelSize: Theme.fontSizeSmall
            color: Theme.primaryColor
            wrapMode: Text.Wrap
            visible: text.length > 0
        }

        DiffView {
            width: parent.width
            diffData: actionData ? actionData.diff : null
            visible: actionData && actionData.diff
        }

        Row {
            anchors.horizontalCenter: parent.horizontalCenter
            spacing: Theme.paddingLarge

            Button {
                text: qsTr("Reject")
                preferredWidth: Theme.buttonWidthSmall
                onClicked: {
                    confirmationCardRoot.confirmed(false)
                }
            }

            Button {
                text: qsTr("Approve & Apply")
                preferredWidth: Theme.buttonWidthSmall
                onClicked: {
                    confirmationCardRoot.confirmed(true)
                }
            }
        }
    }
}
