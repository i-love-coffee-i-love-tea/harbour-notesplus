import QtQuick 2.6
import Sailfish.Silica 1.0
import harbour.notesplus 1.0
import "."

Column {
    id: servicesSettingsTab
    width: parent.width
    spacing: Theme.paddingMedium

    // Properties passed from SettingsPage
    property var page: null  // reference to SettingsPage for shared state

    RemorsePopup {
        id: servicesRemorsePopup
    }

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

    ComboBox {
        id: bindAddressCombo
        width: parent.width
        label: qsTr("Bind to network")
        description: qsTr("Select which network interface the web server listens on")
        currentIndex: {
            var ifaces = page.networkInterfaces
            var current = (typeof app !== "undefined" && app.bindAddress) ? app.bindAddress : "0.0.0.0"
            for (var i = 0; i < ifaces.length; i++) {
                var itemIp = ifaces[i].ip || (Array.isArray(ifaces[i]) ? ifaces[i][0] : "")
                if (itemIp === current) return i
            }
            return 0
        }
        menu: ContextMenu {
            Repeater {
                model: page.networkInterfaces
                MenuItem {
                    text: modelData.name || modelData.ip || (Array.isArray(modelData) ? (modelData[1] || modelData[0]) : "")
                }
            }
        }
        onCurrentIndexChanged: {
            var ifaces = page.networkInterfaces
            if (currentIndex >= 0 && currentIndex < ifaces.length) {
                var selectedIp = ifaces[currentIndex].ip || (Array.isArray(ifaces[currentIndex]) ? ifaces[currentIndex][0] : "")
                if (typeof app !== "undefined" && app && app.setBindAddress && selectedIp && selectedIp !== app.bindAddress) {
                    app.setBindAddress(selectedIp)
                }
            }
        }
        onVisibleChanged: {
            if (visible && page.networkInterfaces.length === 0) {
                page.refreshNetworkInterfaces()
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

    property var serverUrls: {
        if (!bridge.web_server_running) return []
        try { return JSON.parse(bridge.get_server_urls_json()) } catch(e) { return [] }
    }

    ComboBox {
        id: urlPicker
        width: parent.width
        visible: bridge.web_server_running && serverUrls.length > 1
        label: qsTr("Server Address")
        currentIndex: 0
        menu: ContextMenu {
            Repeater {
                model: serverUrls
                MenuItem { text: modelData }
            }
        }
    }

    Button {
        anchors.horizontalCenter: parent.horizontalCenter
        text: qsTr("Copy URL")
        visible: bridge.web_server_running
        onClicked: {
            var urls = serverUrls
            if (urls.length === 0) return
            var idx = urls.length > 1 ? urlPicker.currentIndex : 0
            var url = urls[idx] || urls[0]
            Clipboard.text = url
        }
    }

    SectionHeader {
        text: qsTr("Web Server Authentication")
    }

    ComboBox {
        width: parent.width
        label: qsTr("Session Duration")
        description: qsTr("Validity period for authorized browser sessions")
        currentIndex: {
            var h = (typeof app !== "undefined" && app && app.sessionExpiryHours !== undefined) ? app.sessionExpiryHours : 24
            if (h <= 1) return 0
            if (h <= 4) return 1
            if (h <= 8) return 2
            if (h <= 24) return 3
            if (h <= 168) return 4
            return 5
        }
        menu: ContextMenu {
            MenuItem { text: qsTr("1 Hour") }
            MenuItem { text: qsTr("4 Hours") }
            MenuItem { text: qsTr("8 Hours") }
            MenuItem { text: qsTr("24 Hours (1 Day)") }
            MenuItem { text: qsTr("7 Days (1 Week)") }
            MenuItem { text: qsTr("30 Days (1 Month)") }
        }
        onCurrentIndexChanged: {
            var hours = 24
            if (currentIndex === 0) hours = 1
            else if (currentIndex === 1) hours = 4
            else if (currentIndex === 2) hours = 8
            else if (currentIndex === 3) hours = 24
            else if (currentIndex === 4) hours = 168
            else if (currentIndex === 5) hours = 720
            if (typeof app !== "undefined" && app && app.setSessionExpiryHours) {
                app.setSessionExpiryHours(hours)
            }
        }
    }

    Button {
        anchors.horizontalCenter: parent.horizontalCenter
        text: qsTr("Test Web Login Prompt")
        onClicked: {
            pageStack.push(Qt.resolvedUrl("../../pages/AuthPrompt.qml"), {
                challengeId: "test-challenge",
                verificationCode: "1234"
            })
        }
    }

    SectionHeader {
        text: qsTr("SSL / TLS Certificate")
    }

    Label {
        x: Theme.horizontalPageMargin
        width: parent.width - Theme.horizontalPageMargin * 2
        text: bridge.is_custom_tls_certificate()
            ? qsTr("Custom SSL certificate installed and active")
            : qsTr("Self-signed SSL certificate active")
        color: bridge.is_custom_tls_certificate() ? Theme.highlightColor : Theme.secondaryColor
        font.pixelSize: Theme.fontSizeSmall
        wrapMode: Text.Wrap
    }

    TextField {
        id: certInput
        width: parent.width
        label: qsTr("Certificate (Path or PEM text)")
        labelVisible: true
        placeholderText: qsTr("e.g. /home/defaultuser/cert.pem")
    }

    TextField {
        id: keyInput
        width: parent.width
        label: qsTr("Private Key (Path or PEM text)")
        labelVisible: true
        placeholderText: qsTr("e.g. /home/defaultuser/key.pem")
    }

    Row {
        anchors.horizontalCenter: parent.horizontalCenter
        spacing: Theme.paddingMedium

        Button {
            text: qsTr("Install Certificate")
            enabled: certInput.text.trim().length > 0 && keyInput.text.trim().length > 0
            onClicked: {
                var err = bridge.install_tls_certificate(certInput.text.trim(), keyInput.text.trim())
                if (err) {
                    servicesRemorsePopup.execute("Error: " + err, function() {})
                } else {
                    servicesRemorsePopup.execute("Installed SSL certificate", function() {})
                    certInput.text = ""
                    keyInput.text = ""
                }
            }
        }

        Button {
            text: qsTr("Reset Self-Signed")
            visible: bridge.is_custom_tls_certificate()
            onClicked: {
                var err = bridge.reset_tls_certificate()
                if (err) {
                    servicesRemorsePopup.execute("Error: " + err, function() {})
                } else {
                    servicesRemorsePopup.execute("Reset to self-signed certificate", function() {})
                }
            }
        }
    }

    SectionHeader {
        text: qsTr("Export & Backup")
    }

    Button {
        anchors.horizontalCenter: parent.horizontalCenter
        text: qsTr("Export All Notes as PDF")
        onClicked: {
            var doExportAll = function() {
                var out = bridge.export_all_pdf()
                if (out) {
                    try {
                        var arr = JSON.parse(out)
                        servicesRemorsePopup.execute(qsTr("Exported %1 notes to PDF").arg(arr.length), function() {})
                    } catch (e) {
                        servicesRemorsePopup.execute(qsTr("Exported notes to PDF"), function() {})
                    }
                }
            }

            if (bridge.any_pdf_export_exists()) {
                var dialog = pageStack.push(Qt.resolvedUrl("../../dialogs/ConfirmDialog.qml"), {
                    title: qsTr("Overwrite Existing PDFs?"),
                    message: qsTr("Some notes already have exported PDF files in Notes++ Exports. Do you want to overwrite them?"),
                    acceptText: qsTr("Overwrite")
                })
                dialog.accepted.connect(function() {
                    doExportAll()
                })
            } else {
                doExportAll()
            }
        }
    }

    Button {
        anchors.horizontalCenter: parent.horizontalCenter
        text: qsTr("Export All Notes as HTML5")
        onClicked: {
            var out = bridge.export_all_html()
            if (out) {
                servicesRemorsePopup.execute("Exported to " + out, function() {})
            }
        }
    }

    SectionHeader {
        text: qsTr("About")
    }

    Label {
        x: Theme.horizontalPageMargin
        width: parent.width - Theme.horizontalPageMargin * 2
        text: qsTr("Notes Plus v0.2.0\nAsciiDoc reader & notebook for Sailfish OS")
        color: Theme.secondaryColor
        font.pixelSize: Theme.fontSizeSmall
        wrapMode: Text.Wrap
    }
}
