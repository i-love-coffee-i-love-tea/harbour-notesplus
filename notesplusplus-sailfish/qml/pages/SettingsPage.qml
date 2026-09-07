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
                title: qsTr("Settings")
            }

            // Tab Bar
            Row {
                width: parent.width - Theme.horizontalPageMargin * 2
                anchors.horizontalCenter: parent.horizontalCenter
                spacing: Theme.paddingSmall

                Button {
                    text: qsTr("Display")
                    preferredWidth: (parent.width - Theme.paddingSmall * 2) / 3
                    color: currentTab === 0 ? Theme.highlightColor : Theme.primaryColor
                    backgroundColor: currentTab === 0 ? Theme.rgba(Theme.highlightBackgroundColor, 0.4) : "transparent"
                    onClicked: currentTab = 0
                }

                Button {
                    text: qsTr("AI Assistant")
                    preferredWidth: (parent.width - Theme.paddingSmall * 2) / 3
                    color: currentTab === 1 ? Theme.highlightColor : Theme.primaryColor
                    backgroundColor: currentTab === 1 ? Theme.rgba(Theme.highlightBackgroundColor, 0.4) : "transparent"
                    onClicked: currentTab = 1
                }

                Button {
                    text: qsTr("Services")
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
                    text: qsTr("Font & Typography")
                }

                ComboBox {
                    width: parent.width
                    label: qsTr("Font Family")
                    currentIndex: {
                        switch (app.docFontFamily) {
                            case "serif": return 1
                            case "sans-serif": return 2
                            case "monospace": return 3
                            default: return 0
                        }
                    }
                    menu: ContextMenu {
                        MenuItem { text: qsTr("Sailfish Default") }
                        MenuItem { text: qsTr("Serif") }
                        MenuItem { text: qsTr("Sans-Serif") }
                        MenuItem { text: qsTr("Monospace") }
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
                    label: qsTr("Document Font Size")
                    valueText: {
                        var pct = Math.round(value * 100)
                        if (pct === 100) return qsTr("100% (Default)")
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
                    label: qsTr("Code Font Size")
                    valueText: {
                        var pct = Math.round(value * 100)
                        if (pct === 100) return qsTr("100% (Default)")
                        return pct + "%"
                    }
                    onSliderValueChanged: {
                        app.setCodeFontScale(Math.round(value * 10) / 10)
                    }
                }

                SectionHeader {
                    text: qsTr("Document View")
                }

                Slider {
                    width: parent.width
                    minimumValue: 0
                    maximumValue: 20
                    stepSize: 1
                    value: (typeof app !== "undefined" && app && app.tocCollapseThreshold !== undefined) ? app.tocCollapseThreshold : 5
                    label: qsTr("Collapse Table of Contents")
                    valueText: {
                        var v = Math.round(value)
                        if (v === 0) return qsTr("Always (0+ headings)")
                        if (v >= 20) return qsTr("Never (Always expanded)")
                        return qsTr("When more than %1 headings").arg(v)
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
                    label: qsTr("Preview Scaling Factor")
                    valueText: Math.round(value * 100) + "%"
                    onSliderValueChanged: {
                        if (typeof app !== "undefined" && app && app.setPreviewScale) {
                            app.setPreviewScale(Math.round(value * 100) / 100)
                        }
                    }
                }

                TextSwitch {
                    width: parent.width
                    text: qsTr("Strip Comments")
                    description: qsTr("Standard AsciiDoc behavior drops // comments from rendered documents")
                    checked: (typeof app !== "undefined" && app && app.dropComments !== undefined) ? app.dropComments : true
                    onCheckedChanged: {
                        if (typeof app !== "undefined" && app && app.setDropComments) {
                            app.setDropComments(checked)
                        }
                    }
                }

                TextSwitch {
                    width: parent.width
                    text: qsTr("Allow External Images")
                    description: qsTr("Load images from remote HTTP and HTTPS web addresses")
                    checked: (typeof app !== "undefined" && app && app.allowExternalImages !== undefined) ? app.allowExternalImages : true
                    onCheckedChanged: {
                        if (typeof app !== "undefined" && app && app.setAllowExternalImages) {
                            app.setAllowExternalImages(checked)
                        }
                    }
                }

                TextSwitch {
                    width: parent.width
                    text: qsTr("Enable Daily Journal")
                    description: qsTr("Show daily journal entries and quick capture on the main page")
                    checked: (typeof app !== "undefined" && app && app.journalEnabled !== undefined) ? app.journalEnabled : true
                    onCheckedChanged: {
                        if (typeof app !== "undefined" && app && app.setJournalEnabled) {
                            app.setJournalEnabled(checked)
                        }
                    }
                }

                SectionHeader {
                    text: qsTr("Live Preview")
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
                    text: qsTr("AI Assistant (Ollama / MiMoCode)")
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
                                text: qsTr("Local Ollama")
                                onClicked: {
                                    app.setAiProvider("ollama")
                                    if (endpointField.text === "https://api.mimocode.com") {
                                        endpointField.text = app.defaultAiEndpoint
                                        app.setAiEndpoint(app.defaultAiEndpoint)
                                    }
                                }
                            }
                            MenuItem {
                                text: qsTr("Xiaomi MiMoCode / OpenAI API")
                                onClicked: {
                                    app.setAiProvider("mimocode")
                                    if (endpointField.text === app.defaultAiEndpoint) {
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

                    TextField {
                        id: modelField
                        width: parent.width
                        label: qsTr("Model Name")
                        labelVisible: true
                        placeholderText: qsTr("e.g. llama3.2, qwen2.5, mistral")
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
                        label: qsTr("API Key (Bearer Token)")
                        labelVisible: true
                        placeholderText: app.aiProvider === "ollama" ? qsTr("API Key (optional for local Ollama)") : qsTr("API Key (required for MiMoCode / Cloud APIs)")
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
                        label: qsTr("Request Timeout")
                        valueText: qsTr("%1 seconds").arg(Math.round(value))
                        onSliderValueChanged: {
                            if (typeof app !== "undefined" && app && app.setAiTimeout) {
                                app.setAiTimeout(Math.round(value))
                            }
                        }
                    }

                    TextSwitch {
                        width: parent.width
                        text: qsTr("Auto-Allow Note Reading")
                        description: qsTr("Allow the assistant to search and read note contents automatically")
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
                        description: qsTr("Allow the assistant to create new notes without extra confirmation")
                        checked: app.aiAutoAllowCreate
                        onCheckedChanged: {
                            if (typeof app !== "undefined" && app && app.setAiAutoAllowCreate) {
                                app.setAiAutoAllowCreate(checked)
                            }
                        }
                    }

                    TextSwitch {
                        width: parent.width
                        text: qsTr("Require Confirmation for Edits")
                        description: qsTr("Display line-by-line diff preview and wait for approval before modifying existing notes")
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
                    text: qsTr("Web Server")
                }

                TextSwitch {
                    width: parent.width
                    text: qsTr("Start Web Server on App Launch")
                    description: qsTr("Automatically start documentation service on local WiFi when app starts")
                    checked: (typeof app !== "undefined" && app && app.autostartWebServer !== undefined) ? app.autostartWebServer : false
                    onCheckedChanged: {
                        if (typeof app !== "undefined" && app && app.setAutostartWebServer) {
                            app.setAutostartWebServer(checked)
                        }
                    }
                }

                TextSwitch {
                    width: parent.width
                    text: qsTr("Reject connections from public networks")
                    description: qsTr("Only allow connections from private local networks and reject public Internet access")
                    checked: (typeof app !== "undefined" && app && app.rejectPublicNetworks !== undefined) ? app.rejectPublicNetworks : true
                    onCheckedChanged: {
                        if (typeof app !== "undefined" && app && app.setRejectPublicNetworks) {
                            app.setRejectPublicNetworks(checked)
                        }
                    }
                }

                TextSwitch {
                    width: parent.width
                    text: qsTr("Server Active")
                    description: bridge.web_server_running
                        ? ("Listening at " + bridge.web_server_url)
                        : qsTr("Tap to manually start or stop server")
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
                    text: "Web Server Authentication"
                }

                TextSwitch {
                    width: parent.width
                    text: "Require Authentication"
                    description: "Protect the web portal with password or OpenID Connect SSO"
                    checked: (typeof app !== "undefined" && app && app.webAuthEnabled !== undefined) ? app.webAuthEnabled : false
                    onCheckedChanged: {
                        if (typeof app !== "undefined" && app && app.setWebAuthEnabled) {
                            app.setWebAuthEnabled(checked)
                        }
                    }
                }

                Column {
                    width: parent.width
                    spacing: Theme.paddingMedium
                    visible: (typeof app !== "undefined" && app && app.webAuthEnabled)

                    SectionHeader {
                        text: "HTTP Basic Authentication"
                    }

                    TextSwitch {
                        width: parent.width
                        text: "Enable Basic Auth"
                        description: "Standard local username and password"
                        checked: (typeof app !== "undefined" && app && app.webAuthBasicEnabled !== undefined) ? app.webAuthBasicEnabled : true
                        onCheckedChanged: {
                            if (typeof app !== "undefined" && app && app.setWebAuthBasicEnabled) {
                                app.setWebAuthBasicEnabled(checked)
                            }
                        }
                    }

                    TextField {
                        id: webUserField
                        width: parent.width
                        label: "Username"
                        labelVisible: true
                        placeholderText: "admin"
                        text: (typeof app !== "undefined" && app && app.webAuthUsername) ? app.webAuthUsername : "admin"
                        onTextChanged: {
                            if (typeof app !== "undefined" && app && app.setWebAuthUsername) {
                                app.setWebAuthUsername(text)
                            }
                        }
                    }

                    PasswordField {
                        id: webPassField
                        width: parent.width
                        label: "Password"
                        labelVisible: true
                        placeholderText: "Set new password"
                        text: (typeof app !== "undefined" && app && app.webAuthPassword) ? app.webAuthPassword : ""
                        onTextChanged: {
                            if (typeof app !== "undefined" && app && app.setWebAuthPassword) {
                                app.setWebAuthPassword(text)
                            }
                        }
                    }

                    SectionHeader {
                        text: "OpenID Connect / SSO (OAuth 2.0)"
                    }

                    TextSwitch {
                        width: parent.width
                        text: "Enable OpenID Connect SSO"
                        description: "Single sign-on via Authentik, Keycloak, Google, etc."
                        checked: (typeof app !== "undefined" && app && app.webAuthOauthEnabled !== undefined) ? app.webAuthOauthEnabled : false
                        onCheckedChanged: {
                            if (typeof app !== "undefined" && app && app.setWebAuthOauthEnabled) {
                                app.setWebAuthOauthEnabled(checked)
                            }
                        }
                    }

                    Column {
                        width: parent.width
                        spacing: Theme.paddingMedium
                        visible: (typeof app !== "undefined" && app && app.webAuthOauthEnabled)

                        TextField {
                            id: oauthProviderField
                            width: parent.width
                            label: "Provider Display Name"
                            labelVisible: true
                            placeholderText: "Authentik / Keycloak / Google"
                            text: (typeof app !== "undefined" && app && app.webAuthOauthProviderName) ? app.webAuthOauthProviderName : "Authentik"
                            onTextChanged: {
                                if (typeof app !== "undefined" && app && app.setWebAuthOauthProviderName) {
                                    app.setWebAuthOauthProviderName(text)
                                }
                            }
                        }

                        TextField {
                            id: oauthIssuerField
                            width: parent.width
                            label: "Issuer / Discovery URL"
                            labelVisible: true
                            placeholderText: "https://auth.example.com/application/o/notesplusplus/"
                            text: (typeof app !== "undefined" && app && app.webAuthOauthIssuerUrl) ? app.webAuthOauthIssuerUrl : ""
                            onTextChanged: {
                                if (typeof app !== "undefined" && app && app.setWebAuthOauthIssuerUrl) {
                                    app.setWebAuthOauthIssuerUrl(text)
                                }
                            }
                        }

                        TextField {
                            id: oauthClientIdField
                            width: parent.width
                            label: "Client ID"
                            labelVisible: true
                            placeholderText: "notesplusplus-app"
                            text: (typeof app !== "undefined" && app && app.webAuthOauthClientId) ? app.webAuthOauthClientId : ""
                            onTextChanged: {
                                if (typeof app !== "undefined" && app && app.setWebAuthOauthClientId) {
                                    app.setWebAuthOauthClientId(text)
                                }
                            }
                        }

                        PasswordField {
                            id: oauthClientSecretField
                            width: parent.width
                            label: "Client Secret (optional for PKCE)"
                            labelVisible: true
                            placeholderText: "Leave blank for public PKCE client"
                            text: (typeof app !== "undefined" && app && app.webAuthOauthClientSecret) ? app.webAuthOauthClientSecret : ""
                            onTextChanged: {
                                if (typeof app !== "undefined" && app && app.setWebAuthOauthClientSecret) {
                                    app.setWebAuthOauthClientSecret(text)
                                }
                            }
                        }

                        TextField {
                            id: oauthEmailsField
                            width: parent.width
                            label: "Allowed Emails / Users (comma-separated)"
                            labelVisible: true
                            placeholderText: "user@example.com, *@mycompany.com"
                            text: (typeof app !== "undefined" && app && app.webAuthOauthAllowedEmails) ? app.webAuthOauthAllowedEmails : ""
                            onTextChanged: {
                                if (typeof app !== "undefined" && app && app.setWebAuthOauthAllowedEmails) {
                                    app.setWebAuthOauthAllowedEmails(text)
                                }
                            }
                        }

                        TextSwitch {
                            width: parent.width
                            text: "Accept Self-Signed IdP Certificates"
                            description: "Allow connecting to self-hosted IdPs with self-signed SSL"
                            checked: (typeof app !== "undefined" && app && app.webAuthOauthAllowSelfSigned !== undefined) ? app.webAuthOauthAllowSelfSigned : false
                            onCheckedChanged: {
                                if (typeof app !== "undefined" && app && app.setWebAuthOauthAllowSelfSigned) {
                                    app.setWebAuthOauthAllowSelfSigned(checked)
                                }
                            }
                        }
                    }
                }

                SectionHeader {
                    text: "SSL / TLS Certificate"
                }

                Label {
                    x: Theme.horizontalPageMargin
                    width: parent.width - Theme.horizontalPageMargin * 2
                    text: bridge.is_custom_tls_certificate()
                        ? "Custom SSL certificate installed and active"
                        : "Self-signed SSL certificate active"
                    color: bridge.is_custom_tls_certificate() ? Theme.highlightColor : Theme.secondaryColor
                    font.pixelSize: Theme.fontSizeSmall
                    wrapMode: Text.Wrap
                }

                TextField {
                    id: certInput
                    width: parent.width
                    label: "Certificate (Path or PEM text)"
                    labelVisible: true
                    placeholderText: "e.g. /home/defaultuser/cert.pem"
                }

                TextField {
                    id: keyInput
                    width: parent.width
                    label: "Private Key (Path or PEM text)"
                    labelVisible: true
                    placeholderText: "e.g. /home/defaultuser/key.pem"
                }

                Row {
                    anchors.horizontalCenter: parent.horizontalCenter
                    spacing: Theme.paddingMedium

                    Button {
                        text: "Install Certificate"
                        enabled: certInput.text.trim().length > 0 && keyInput.text.trim().length > 0
                        onClicked: {
                            var err = bridge.install_tls_certificate(certInput.text.trim(), keyInput.text.trim())
                            if (err) {
                                remorsePopup.execute("Error: " + err, function() {})
                            } else {
                                remorsePopup.execute("Installed SSL certificate", function() {})
                                certInput.text = ""
                                keyInput.text = ""
                            }
                        }
                    }

                    Button {
                        text: "Reset Self-Signed"
                        visible: bridge.is_custom_tls_certificate()
                        onClicked: {
                            var err = bridge.reset_tls_certificate()
                            if (err) {
                                remorsePopup.execute("Error: " + err, function() {})
                            } else {
                                remorsePopup.execute("Reset to self-signed certificate", function() {})
                            }
                        }
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
