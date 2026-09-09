import QtQuick 2.6
import Sailfish.Silica 1.0

Page {
    id: aiCapabilitiesPage
    allowedOrientations: Orientation.All

    SilicaFlickable {
        anchors.fill: parent
        contentHeight: contentCol.height + Theme.paddingLarge

        Column {
            id: contentCol
            width: parent.width
            spacing: Theme.paddingMedium

            PageHeader {
                title: qsTr("What Can the AI Do?")
            }

            // Intro
            Label {
                x: Theme.horizontalPageMargin
                width: parent.width - Theme.horizontalPageMargin * 2
                text: qsTr("When you chat with the AI assistant, it can work directly with your notes. This page explains what it is allowed to do, and how you stay in control.")
                color: Theme.primaryColor
                font.pixelSize: Theme.fontSizeSmall
                wrapMode: Text.Wrap
            }

            // ---- READING ----
            SectionHeader {
                text: qsTr("Reading Your Notes")
            }

            Rectangle {
                width: parent.width - Theme.horizontalPageMargin * 2
                anchors.horizontalCenter: parent.horizontalCenter
                height: readCol.height + Theme.paddingMedium * 2
                color: Theme.rgba(Theme.highlightBackgroundColor, 0.1)
                border.color: Theme.rgba(Theme.highlightColor, 0.3)
                border.width: 1
                radius: Theme.paddingSmall

                Column {
                    id: readCol
                    anchors {
                        left: parent.left
                        right: parent.right
                        top: parent.top
                        margins: Theme.paddingMedium
                    }
                    spacing: Theme.paddingSmall

                    Label {
                        width: parent.width
                        text: qsTr("The AI can read, search, and list your notes to answer questions or find information.")
                        color: Theme.primaryColor
                        font.pixelSize: Theme.fontSizeSmall
                        wrapMode: Text.Wrap
                    }

                    Label {
                        width: parent.width
                        text: qsTr("For example, you can ask \"What did I write about the garden project?\" and the AI will search your notes to find relevant content.")
                        color: Theme.secondaryColor
                        font.pixelSize: Theme.fontSizeExtraSmall
                        wrapMode: Text.Wrap
                    }

                    Label {
                        width: parent.width
                        text: qsTr("By default, reading is allowed automatically. You can turn this off in the permissions above — the AI will then ask you each time before looking at a note.")
                        color: Theme.secondaryColor
                        font.pixelSize: Theme.fontSizeExtraSmall
                        wrapMode: Text.Wrap
                    }
                }
            }

            // ---- CREATING ----
            SectionHeader {
                text: qsTr("Creating Notes")
            }

            Rectangle {
                width: parent.width - Theme.horizontalPageMargin * 2
                anchors.horizontalCenter: parent.horizontalCenter
                height: createCol.height + Theme.paddingMedium * 2
                color: Theme.rgba(Theme.highlightBackgroundColor, 0.1)
                border.color: Theme.rgba(Theme.highlightColor, 0.3)
                border.width: 1
                radius: Theme.paddingSmall

                Column {
                    id: createCol
                    anchors {
                        left: parent.left
                        right: parent.right
                        top: parent.top
                        margins: Theme.paddingMedium
                    }
                    spacing: Theme.paddingSmall

                    Label {
                        width: parent.width
                        text: qsTr("The AI can create new notes for you. For example, you can ask it to draft a summary, write meeting notes, or save a recipe from a webpage.")
                        color: Theme.primaryColor
                        font.pixelSize: Theme.fontSizeSmall
                        wrapMode: Text.Wrap
                    }

                    Label {
                        width: parent.width
                        text: qsTr("New notes appear in your library like any other note. By default, the AI can create notes without asking — you can change this in the permissions above.")
                        color: Theme.secondaryColor
                        font.pixelSize: Theme.fontSizeExtraSmall
                        wrapMode: Text.Wrap
                    }
                }
            }

            // ---- EDITING ----
            SectionHeader {
                text: qsTr("Editing Notes")
            }

            Rectangle {
                width: parent.width - Theme.horizontalPageMargin * 2
                anchors.horizontalCenter: parent.horizontalCenter
                height: editCol.height + Theme.paddingMedium * 2
                color: Theme.rgba(Theme.highlightBackgroundColor, 0.1)
                border.color: Theme.rgba(Theme.highlightColor, 0.3)
                border.width: 1
                radius: Theme.paddingSmall

                Column {
                    id: editCol
                    anchors {
                        left: parent.left
                        right: parent.right
                        top: parent.top
                        margins: Theme.paddingMedium
                    }
                    spacing: Theme.paddingSmall

                    Label {
                        width: parent.width
                        text: qsTr("The AI can change the content of existing notes. For example, it can fix spelling, reformat text, or add a section you asked for.")
                        color: Theme.primaryColor
                        font.pixelSize: Theme.fontSizeSmall
                        wrapMode: Text.Wrap
                    }

                    Label {
                        width: parent.width
                        text: qsTr("Before any edit is applied, you will always see exactly what changed — line by line — and can approve or reject it. This confirmation step cannot be turned off.")
                        color: Theme.highlightColor
                        font.pixelSize: Theme.fontSizeExtraSmall
                        font.bold: true
                        wrapMode: Text.Wrap
                    }

                    Label {
                        width: parent.width
                        text: qsTr("If you approve an edit and later change your mind, you can undo it from the assistant chat screen.")
                        color: Theme.secondaryColor
                        font.pixelSize: Theme.fontSizeExtraSmall
                        wrapMode: Text.Wrap
                    }
                }
            }

            // ---- FETCHING URLS ----
            SectionHeader {
                text: qsTr("Fetching Web Content")
            }

            Rectangle {
                width: parent.width - Theme.horizontalPageMargin * 2
                anchors.horizontalCenter: parent.horizontalCenter
                height: fetchCol.height + Theme.paddingMedium * 2
                color: Theme.rgba(Theme.highlightBackgroundColor, 0.1)
                border.color: Theme.rgba(Theme.highlightColor, 0.3)
                border.width: 1
                radius: Theme.paddingSmall

                Column {
                    id: fetchCol
                    anchors {
                        left: parent.left
                        right: parent.right
                        top: parent.top
                        margins: Theme.paddingMedium
                    }
                    spacing: Theme.paddingSmall

                    Label {
                        width: parent.width
                        text: qsTr("The AI can visit a webpage you give it and read its text content. This is useful for summarizing articles, extracting information, or importing content as a new note.")
                        color: Theme.primaryColor
                        font.pixelSize: Theme.fontSizeSmall
                        wrapMode: Text.Wrap
                    }

                    Label {
                        width: parent.width
                        text: qsTr("The AI cannot browse the web on its own — it can only fetch URLs you specifically mention in your message.")
                        color: Theme.secondaryColor
                        font.pixelSize: Theme.fontSizeExtraSmall
                        wrapMode: Text.Wrap
                    }

                    Label {
                        width: parent.width
                        text: qsTr("For security, the AI is blocked from accessing local network addresses (like your router or other devices on your WiFi).")
                        color: Theme.secondaryColor
                        font.pixelSize: Theme.fontSizeExtraSmall
                        wrapMode: Text.Wrap
                    }

                    Label {
                        width: parent.width
                        text: qsTr("You can turn off web requests entirely in the tool permissions above.")
                        color: Theme.secondaryColor
                        font.pixelSize: Theme.fontSizeExtraSmall
                        wrapMode: Text.Wrap
                    }
                }
            }

            // ---- PRIVACY NOTE ----
            SectionHeader {
                text: qsTr("Privacy")
            }

            Rectangle {
                width: parent.width - Theme.horizontalPageMargin * 2
                anchors.horizontalCenter: parent.horizontalCenter
                height: privacyCol.height + Theme.paddingMedium * 2
                color: Theme.rgba(Theme.highlightBackgroundColor, 0.1)
                border.color: Theme.rgba(Theme.highlightColor, 0.3)
                border.width: 1
                radius: Theme.paddingSmall

                Column {
                    id: privacyCol
                    anchors {
                        left: parent.left
                        right: parent.right
                        top: parent.top
                        margins: Theme.paddingMedium
                    }
                    spacing: Theme.paddingSmall

                    Label {
                        width: parent.width
                        text: qsTr("When you use Ollama, your notes are sent to the computer running Ollama — typically your own laptop or home server on your local network. Nothing goes to the cloud.")
                        color: Theme.primaryColor
                        font.pixelSize: Theme.fontSizeSmall
                        wrapMode: Text.Wrap
                    }

                    Label {
                        width: parent.width
                        text: qsTr("When you use a cloud provider (MiMoCode or other OpenAI-compatible APIs), the content of your messages and any notes the AI reads are sent to that provider's servers for processing. Only the notes the AI actually reads are sent — not your entire library.")
                        color: Theme.secondaryColor
                        font.pixelSize: Theme.fontSizeExtraSmall
                        wrapMode: Text.Wrap
                    }
                }
            }
        }
    }
}
