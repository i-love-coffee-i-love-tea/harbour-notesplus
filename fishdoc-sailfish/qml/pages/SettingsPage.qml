import QtQuick 2.6
import Sailfish.Silica 1.0
import "../components"

Page {
    id: settingsPage
    allowedOrientations: Orientation.All

    property int currentTab: 0

    SilicaFlickable {
        anchors.fill: parent
        contentHeight: column.height + Theme.paddingLarge

        Column {
            id: column
            width: parent.width
            spacing: Theme.paddingMedium

            PageHeader {
                title: "Settings"
            }

            // Tab Bar
            Row {
                width: parent.width - Theme.horizontalPageMargin * 2
                anchors.horizontalCenter: parent.horizontalCenter
                spacing: Theme.paddingSmall

                Button {
                    text: "Display"
                    preferredWidth: (parent.width - Theme.paddingSmall * 2) / 3
                    color: currentTab === 0 ? Theme.highlightColor : Theme.primaryColor
                    backgroundColor: currentTab === 0 ? Theme.rgba(Theme.highlightBackgroundColor, 0.4) : "transparent"
                    onClicked: currentTab = 0
                }

                Button {
                    text: "AI Assistant"
                    preferredWidth: (parent.width - Theme.paddingSmall * 2) / 3
                    color: currentTab === 1 ? Theme.highlightColor : Theme.primaryColor
                    backgroundColor: currentTab === 1 ? Theme.rgba(Theme.highlightBackgroundColor, 0.4) : "transparent"
                    onClicked: currentTab = 1
                }

                Button {
                    text: "Services"
                    preferredWidth: (parent.width - Theme.paddingSmall * 2) / 3
                    color: currentTab === 2 ? Theme.highlightColor : Theme.primaryColor
                    backgroundColor: currentTab === 2 ? Theme.rgba(Theme.highlightBackgroundColor, 0.4) : "transparent"
                    onClicked: currentTab = 2
                }
            }

            // ==================== TAB 0: DISPLAY & FONTS ====================
            Column {
                id: displayTabCol
                width: parent.width
                spacing: Theme.paddingMedium
                visible: currentTab === 0

                SectionHeader {
                    text: "Font & Typography"
                }

                ComboBox {
                    width: parent.width
                    label: "Font Family"
                    currentIndex: {
                        switch (app.docFontFamily) {
                            case "serif": return 1
                            case "sans-serif": return 2
                            case "monospace": return 3
                            default: return 0
                        }
                    }
                    menu: ContextMenu {
                        MenuItem { text: "Sailfish Default" }
                        MenuItem { text: "Serif" }
                        MenuItem { text: "Sans-Serif" }
                        MenuItem { text: "Monospace" }
                    }
                    onCurrentIndexChanged: {
                        switch (currentIndex) {
                            case 1: app.setFontFamily("serif"); break
                            case 2: app.setFontFamily("sans-serif"); break
                            case 3: app.setFontFamily("monospace"); break
                            default: app.setFontFamily(""); break
                        }
                    }
                }

                Slider {
                    width: parent.width
                    minimumValue: 0.8
                    maximumValue: 1.5
                    stepSize: 0.1
                    value: app.fontScale
                    label: "Document Font Size"
                    valueText: {
                        var pct = Math.round(value * 100)
                        if (pct === 100) return "100% (Default)"
                        return pct + "%"
                    }
                    onSliderValueChanged: {
                        app.setFontScale(Math.round(value * 10) / 10)
                    }
                }

                Slider {
                    width: parent.width
                    minimumValue: 0.8
                    maximumValue: 1.5
                    stepSize: 0.1
                    value: app.codeFontScale
                    label: "Code Font Size"
                    valueText: {
                        var pct = Math.round(value * 100)
                        if (pct === 100) return "100% (Default)"
                        return pct + "%"
                    }
                    onSliderValueChanged: {
                        app.setCodeFontScale(Math.round(value * 10) / 10)
                    }
                }

                SectionHeader {
                    text: "Document View"
                }

                Slider {
                    width: parent.width
                    minimumValue: 0
                    maximumValue: 20
                    stepSize: 1
                    value: (typeof app !== "undefined" && app && app.tocCollapseThreshold !== undefined) ? app.tocCollapseThreshold : 5
                    label: "Collapse Table of Contents"
                    valueText: {
                        var v = Math.round(value)
                        if (v === 0) return "Always (0+ headings)"
                        if (v >= 20) return "Never (Always expanded)"
                        return "When more than " + v + " headings"
                    }
                    onSliderValueChanged: {
                        if (typeof app !== "undefined" && app && app.setTocCollapseThreshold) {
                            app.setTocCollapseThreshold(Math.round(value))
                        }
                    }
                }

                Slider {
                    width: parent.width
                    minimumValue: 0.3
                    maximumValue: 0.8
                    stepSize: 0.02
                    value: (typeof app !== "undefined" && app && app.previewScale !== undefined) ? app.previewScale : 0.52
                    label: "Preview Scaling Factor"
                    valueText: Math.round(value * 100) + "%"
                    onSliderValueChanged: {
                        if (typeof app !== "undefined" && app && app.setPreviewScale) {
                            app.setPreviewScale(Math.round(value * 100) / 100)
                        }
                    }
                }

                TextSwitch {
                    width: parent.width
                    text: "Strip Comments"
                    description: "Standard AsciiDoc behavior drops // comments from rendered documents"
                    checked: (typeof app !== "undefined" && app && app.dropComments !== undefined) ? app.dropComments : true
                    onCheckedChanged: {
                        if (typeof app !== "undefined" && app && app.setDropComments) {
                            app.setDropComments(checked)
                        }
                    }
                }

                TextSwitch {
                    width: parent.width
                    text: "Allow External Images"
                    description: "Load images from remote HTTP and HTTPS web addresses"
                    checked: (typeof app !== "undefined" && app && app.allowExternalImages !== undefined) ? app.allowExternalImages : true
                    onCheckedChanged: {
                        if (typeof app !== "undefined" && app && app.setAllowExternalImages) {
                            app.setAllowExternalImages(checked)
                        }
                    }
                }

                SectionHeader {
                    text: "Live Preview"
                }

                Rectangle {
                    width: parent.width - Theme.horizontalPageMargin * 2
                    x: Theme.horizontalPageMargin
                    height: previewCol.height + Theme.paddingMedium * 2
                    color: Theme.rgba(Theme.highlightBackgroundColor, 0.08)
                    border.color: Theme.rgba(Theme.primaryColor, 0.15)
                    border.width: 1
                    radius: Theme.paddingSmall

                    Column {
                        id: previewCol
                        anchors {
                            left: parent.left
                            right: parent.right
                            top: parent.top
                            margins: Theme.paddingMedium
                        }
                        spacing: Theme.paddingSmall

                        Label {
                            width: parent.width
                            text: "Sample Document Heading"
                            color: Theme.highlightColor
                            font.bold: true
                            font.family: (app.docFontFamily && app.docFontFamily.length > 0) ? app.docFontFamily : Theme.fontFamily
                            font.pixelSize: Math.round(Theme.fontSizeLarge * app.fontScale)
                            wrapMode: Text.Wrap
                        }

                        Label {
                            width: parent.width
                            text: "This is a live preview of body text with <b>bold</b>, <i>italic</i>, and <code style='background:#18181c;color:#f2f2f7;padding:1px 4px;border-radius:3px;font-family:monospace;'>code spans</code>."
                            textFormat: Text.RichText
                            color: Theme.primaryColor
                            font.family: (app.docFontFamily && app.docFontFamily.length > 0) ? app.docFontFamily : Theme.fontFamily
                            font.pixelSize: Math.round(Theme.fontSizeMedium * app.fontScale)
                            wrapMode: Text.Wrap
                        }

                        Rectangle {
                            width: parent.width
                            color: "#18181c"
                            border.color: Theme.rgba(Theme.primaryColor, 0.2)
                            border.width: 1
                            radius: Theme.paddingSmall
                            height: previewCodeCol.height + Theme.paddingMedium * 2

                            Column {
                                id: previewCodeCol
                                anchors {
                                    left: parent.left
                                    right: parent.right
                                    top: parent.top
                                    margins: Theme.paddingMedium
                                }
                                spacing: Theme.paddingSmall / 2

                                Label {
                                    text: "RUST"
                                    font.family: "monospace"
                                    font.pixelSize: Math.round(Theme.fontSizeExtraSmall * app.codeFontScale)
                                    color: Theme.rgba("#f2f2f7", 0.6)
                                }

                                Label {
                                    width: parent.width
                                    text: "fn main() {\n    println!(\"Hello Notes++!\");\n}"
                                    font.family: "monospace"
                                    font.pixelSize: Math.round(Theme.fontSizeSmall * app.codeFontScale)
                                    color: "#f2f2f7"
                                    wrapMode: Text.Wrap
                                }
                            }
                        }

                        Label {
                            text: "Card Miniature"
                            color: Theme.highlightColor
                            font.pixelSize: Theme.fontSizeExtraSmall
                            font.bold: true
                        }

                        MiniDocPreview {
                            width: parent.width
                            previewHeight: width * 0.6
                            previewBlocksJson: JSON.stringify([
                                { type: "heading", level: 1, spans: [{ type: "text", value: "Document Title" }] },
                                { type: "toc", headings: [{ level: 1, text: "Document Title" }, { level: 2, text: "Getting Started" }, { level: 2, text: "Usage Guide" }] },
                                { type: "paragraph", spans: [{ type: "text", value: "Preview demonstrates scaling and TOC settings." }] },
                                { type: "unordered_list_item", checked: true, blocks: [{ type: "paragraph", spans: [{ type: "text", value: "Configurable scaling" }] }] }
                            ])
                        }
                    }
                }
            }

            // ==================== TAB 1: AI ASSISTANT ====================
            Column {
                id: aiTabCol
                width: parent.width
                spacing: Theme.paddingMedium
                visible: currentTab === 1

                SectionHeader {
                    text: "AI Assistant (Ollama / MiMoCode)"
                }

                TextSwitch {
                    width: parent.width
                    text: "Enable AI Features"
                    description: "Enable AI Assistant, action templates, and note import tools"
                    checked: (typeof app !== "undefined" && app && app.aiEnabled !== undefined) ? app.aiEnabled : true
                    onCheckedChanged: {
                        if (typeof app !== "undefined" && app && app.setAiEnabled) {
                            app.setAiEnabled(checked)
                        }
                    }
                }

                Column {
                    width: parent.width
                    spacing: Theme.paddingMedium
                    visible: (typeof app !== "undefined" && app && app.aiEnabled !== undefined) ? app.aiEnabled : true

                    ComboBox {
                        width: parent.width
                        label: "AI Provider"
                        currentIndex: (app.aiProvider === "mimocode" || app.aiProvider === "openai") ? 1 : 0
                        menu: ContextMenu {
                            MenuItem {
                                text: "Local Ollama"
                                onClicked: {
                                    app.setAiProvider("ollama")
                                    if (endpointField.text === "https://api.mimocode.com") {
                                        endpointField.text = "http://192.168.1.1:11434"
                                        app.setAiEndpoint("http://192.168.1.1:11434")
                                    }
                                }
                            }
                            MenuItem {
                                text: "Xiaomi MiMoCode / OpenAI API"
                                onClicked: {
                                    app.setAiProvider("mimocode")
                                    if (endpointField.text === "http://192.168.1.1:11434") {
                                        endpointField.text = "https://api.mimocode.com"
                                        app.setAiEndpoint("https://api.mimocode.com")
                                    }
                                }
                            }
                        }
                    }

                    TextField {
                        id: endpointField
                        width: parent.width
                        label: "Endpoint URL"
                        placeholderText: app.aiProvider === "ollama" ? "http://192.168.1.1:11434" : "https://api.mimocode.com"
                        text: app.aiEndpoint
                        onTextChanged: {
                            if (typeof app !== "undefined" && app && app.setAiEndpoint) {
                                app.setAiEndpoint(text)
                            }
                        }
                    }

                    TextField {
                        id: modelField
                        width: parent.width
                        label: "Model Name"
                        placeholderText: "e.g. llama3.2, qwen2.5, mistral"
                        text: app.aiModel
                        onTextChanged: {
                            if (typeof app !== "undefined" && app && app.setAiModel) {
                                app.setAiModel(text)
                            }
                        }
                    }

                    PasswordField {
                        id: apiKeyField
                        width: parent.width
                        label: "API Key (Bearer Token)"
                        placeholderText: app.aiProvider === "ollama" ? "Optional for local Ollama" : "Required for MiMoCode / Cloud APIs"
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
                        maximumValue: 300
                        stepSize: 15
                        value: (typeof app !== "undefined" && app && app.aiTimeout !== undefined) ? app.aiTimeout : 90
                        label: "Request Timeout"
                        valueText: Math.round(value) + " seconds"
                        onSliderValueChanged: {
                            if (typeof app !== "undefined" && app && app.setAiTimeout) {
                                app.setAiTimeout(Math.round(value))
                            }
                        }
                    }

                    TextSwitch {
                        width: parent.width
                        text: "Auto-Allow Note Reading"
                        description: "Allow the assistant to search and read note contents automatically"
                        checked: app.aiAutoAllowRead
                        onCheckedChanged: {
                            if (typeof app !== "undefined" && app && app.setAiAutoAllowRead) {
                                app.setAiAutoAllowRead(checked)
                            }
                        }
                    }

                    TextSwitch {
                        width: parent.width
                        text: "Auto-Allow Note Creation"
                        description: "Allow the assistant to create new notes without extra confirmation"
                        checked: app.aiAutoAllowCreate
                        onCheckedChanged: {
                            if (typeof app !== "undefined" && app && app.setAiAutoAllowCreate) {
                                app.setAiAutoAllowCreate(checked)
                            }
                        }
                    }

                    TextSwitch {
                        width: parent.width
                        text: "Require Confirmation for Edits"
                        description: "Display line-by-line diff preview and wait for approval before modifying existing notes"
                        checked: app.aiRequireConfirmEdit
                        onCheckedChanged: {
                            if (typeof app !== "undefined" && app && app.setAiRequireConfirmEdit) {
                                app.setAiRequireConfirmEdit(checked)
                            }
                        }
                    }

                    Button {
                        anchors.horizontalCenter: parent.horizontalCenter
                        text: "Open AI Assistant"
                        onClicked: {
                            app.openAssistant("", "")
                        }
                    }
                }
            }

            // ==================== TAB 2: SERVICES & BACKUP ====================
            Column {
                id: servicesTabCol
                width: parent.width
                spacing: Theme.paddingMedium
                visible: currentTab === 2

                SectionHeader {
                    text: "Web Server"
                }

                TextSwitch {
                    width: parent.width
                    text: "Start Web Server on App Launch"
                    description: "Automatically start documentation service on local WiFi when app starts"
                    checked: (typeof app !== "undefined" && app && app.autostartWebServer !== undefined) ? app.autostartWebServer : false
                    onCheckedChanged: {
                        if (typeof app !== "undefined" && app && app.setAutostartWebServer) {
                            app.setAutostartWebServer(checked)
                        }
                    }
                }

                TextSwitch {
                    width: parent.width
                    text: "Server Active"
                    description: bridge.web_server_running
                        ? ("Listening at " + bridge.web_server_url)
                        : "Tap to manually start or stop server"
                    checked: bridge.web_server_running
                    onClicked: {
                        bridge.toggle_web_server()
                    }
                }

                Button {
                    anchors.horizontalCenter: parent.horizontalCenter
                    text: "Open Web Portal in Browser"
                    visible: bridge.web_server_running
                    onClicked: {
                        bridge.open_in_browser("")
                    }
                }

                SectionHeader {
                    text: "Export & Backup"
                }

                Button {
                    anchors.horizontalCenter: parent.horizontalCenter
                    text: "Export All Notes as HTML5"
                    onClicked: {
                        var out = bridge.export_all_html()
                        if (out) {
                            remorsePopup.execute("Exported to " + out, function() {})
                        }
                    }
                }

                SectionHeader {
                    text: "About"
                }

                Label {
                    x: Theme.horizontalPageMargin
                    width: parent.width - Theme.horizontalPageMargin * 2
                    text: "Notes++ v0.1.0\nAsciiDoc reader & notebook for Sailfish OS"
                    color: Theme.secondaryColor
                    font.pixelSize: Theme.fontSizeSmall
                    wrapMode: Text.Wrap
                }
            }
        }
    }

    RemorsePopup {
        id: remorsePopup
    }
}
