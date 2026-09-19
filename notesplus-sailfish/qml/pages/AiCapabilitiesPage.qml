import QtQuick 2.6
import Sailfish.Silica 1.0
import "../components/common"

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
                text: qsTr("Reading & Context Retrieval")
            }

            InfoCard {
                Label {
                    width: parent.width
                    text: qsTr("The AI can read, search, and list your notes, as well as retrieve relevant snippets automatically using the local SQLite FTS5 full-text index.")
                    color: Theme.primaryColor
                    font.pixelSize: Theme.fontSizeSmall
                    wrapMode: Text.Wrap
                }

                Label {
                    width: parent.width
                    text: qsTr("For example, you can ask \"Summarize what I planned for Q4\" and the assistant uses hybrid retrieval across your note library to find relevant sections without needing manual attachments.")
                    color: Theme.secondaryColor
                    font.pixelSize: Theme.fontSizeExtraSmall
                    wrapMode: Text.Wrap
                }

                Label {
                    width: parent.width
                    text: qsTr("Tools used: read_note, list_notes, search_notes, retrieve_context, list_groups.")
                    color: Theme.highlightColor
                    font.pixelSize: Theme.fontSizeExtraSmall - 2
                    wrapMode: Text.Wrap
                }
            }

            // ---- CREATING ----
            SectionHeader {
                text: qsTr("Creating Notes")
            }

            InfoCard {
                Label {
                    width: parent.width
                    text: qsTr("The AI can create new notes in valid AsciiDoc format. For example, you can ask it to draft a meeting summary, outline a project, or save recipes.")
                    color: Theme.primaryColor
                    font.pixelSize: Theme.fontSizeSmall
                    wrapMode: Text.Wrap
                }

                Label {
                    width: parent.width
                    text: qsTr("New notes are saved directly into your note directory and automatically indexed for instant search.")
                    color: Theme.secondaryColor
                    font.pixelSize: Theme.fontSizeExtraSmall
                    wrapMode: Text.Wrap
                }

                Label {
                    width: parent.width
                    text: qsTr("Tool used: create_note.")
                    color: Theme.highlightColor
                    font.pixelSize: Theme.fontSizeExtraSmall - 2
                    wrapMode: Text.Wrap
                }
            }

            // ---- EDITING ----
            SectionHeader {
                text: qsTr("Granular Section Editing & Appending")
            }

            InfoCard {
                Label {
                    width: parent.width
                    text: qsTr("The AI can modify existing notes either in full or by targeting specific AsciiDoc sections without touching the rest of your document.")
                    color: Theme.primaryColor
                    font.pixelSize: Theme.fontSizeSmall
                    wrapMode: Text.Wrap
                }

                Label {
                    width: parent.width
                    text: qsTr("Specialized operations:\n• edit_section: Replaces or renames a single section by heading.\n• append_to_note: Appends checklist items or text to the end of a note or section.\n• insert_section: Inserts new AsciiDoc sections before or after existing headings.\n• edit_note: Replaces full document content when comprehensive restructuring is needed.")
                    color: Theme.secondaryColor
                    font.pixelSize: Theme.fontSizeExtraSmall
                    wrapMode: Text.Wrap
                }

                Label {
                    width: parent.width
                    text: qsTr("Before any modification is applied, you will see a focused line-by-line diff preview to approve or reject. Automatic backup snapshots allow one-tap rollbacks.")
                    color: Theme.primaryColor
                    font.pixelSize: Theme.fontSizeExtraSmall
                    font.bold: true
                    wrapMode: Text.Wrap
                }
            }

            // ---- GROUPS & ORGANIZATION ----
            SectionHeader {
                text: qsTr("Groups & Note Organization")
            }

            InfoCard {
                Label {
                    width: parent.width
                    text: qsTr("The AI understands hierarchical note groups and folders. You can ask it to explore categories or relocate notes between groups (e.g., move meeting notes to 'Work/Projects' or archive old drafts).")
                    color: Theme.primaryColor
                    font.pixelSize: Theme.fontSizeSmall
                    wrapMode: Text.Wrap
                }

                Label {
                    width: parent.width
                    text: qsTr("Moving a note automatically updates all incoming and outgoing cross-references (`xref:...`) across your library so links never break.")
                    color: Theme.secondaryColor
                    font.pixelSize: Theme.fontSizeExtraSmall
                    wrapMode: Text.Wrap
                }

                Label {
                    width: parent.width
                    text: qsTr("Tools used: list_groups, move_note.")
                    color: Theme.highlightColor
                    font.pixelSize: Theme.fontSizeExtraSmall - 2
                    wrapMode: Text.Wrap
                }
            }

            // ---- FETCHING URLS ----
            SectionHeader {
                text: qsTr("Fetching Web Content")
            }

            InfoCard {
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
                    text: qsTr("Tool used: fetch_url.")
                    color: Theme.highlightColor
                    font.pixelSize: Theme.fontSizeExtraSmall - 2
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

            // ---- PRIVACY NOTE ----
            SectionHeader {
                text: qsTr("Privacy")
            }

            InfoCard {
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

            // ---- PERMISSION MAPPING ----
            SectionHeader {
                text: qsTr("Permission Controls")
            }

            InfoCard {
                Label {
                    width: parent.width
                    text: qsTr("Each setting in Settings > AI Assistant > Tool Permissions controls a group of tools. Here is the full mapping:")
                    color: Theme.primaryColor
                    font.pixelSize: Theme.fontSizeSmall
                    wrapMode: Text.Wrap
                }

                Label {
                    width: parent.width
                    text: qsTr("Auto-Allow Reading → read_note, list_notes, search_notes, retrieve_context, list_groups\n\nAuto-Allow Creation → create_note\n\nRequire Confirmation for Edits → edit_note, edit_section, append_to_note, insert_section, move_note\n\nAllow Web Requests → fetch_url")
                    color: Theme.secondaryColor
                    font.pixelSize: Theme.fontSizeExtraSmall
                    wrapMode: Text.Wrap
                }
            }
        }
    }
}
