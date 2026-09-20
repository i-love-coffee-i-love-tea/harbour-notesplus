import QtQuick 2.6
import Sailfish.Silica 1.0
import harbour.notesplus 1.0

Page {
    id: webSettingsPage
    allowedOrientations: Orientation.All

    property var networkInterfaces: []

    function refreshNetworkInterfaces() {
        try {
            var json = bridge.get_network_interfaces_json()
            networkInterfaces = JSON.parse(json) || []
        } catch (e) {
            networkInterfaces = []
        }
    }

    Component.onCompleted: {
        refreshNetworkInterfaces()
    }

    RemorsePopup {
        id: webRemorsePopup
    }

    SilicaFlickable {
        anchors.fill: parent
        contentHeight: column.height + Theme.paddingLarge

        Column {
            id: column
            width: parent.width
            spacing: Theme.paddingMedium

            PageHeader {
                title: qsTr("Web Server")
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
                checked: (typeof app !== "undefined" && app && app.rejectPublicNetworks !== undefined) ? app.rejectPublicNetworks : false
                onCheckedChanged: {
                    if (typeof app !== "undefined" && app && app.setRejectPublicNetworks) {
                        app.setRejectPublicNetworks(checked)
                    }
                }
            }

            ComboBox {
                id: interfaceComboBox
                width: parent.width
                label: qsTr("Bind to Network Interface")
                description: qsTr("Choose which network interface the web server listens on. Defaults to all private local interfaces.")
                currentIndex: {
                    var currentBind = (typeof app !== "undefined" && app && app.bindAddress) ? app.bindAddress : ""
                    if (!currentBind || currentBind === "") return 0
                    for (var i = 0; i < networkInterfaces.length; i++) {
                        if (networkInterfaces[i].ip === currentBind) return i + 1
                    }
                    return 0
                }
                menu: ContextMenu {
                    MenuItem {
                        text: qsTr("All private local networks (Default)")
                        onClicked: {
                            if (typeof app !== "undefined" && app && app.setBindAddress) {
                                app.setBindAddress("")
                            }
                        }
                    }
                    Repeater {
                        model: networkInterfaces
                        MenuItem {
                            text: modelData.name + " (" + modelData.ip + ")"
                            onClicked: {
                                if (typeof app !== "undefined" && app && app.setBindAddress) {
                                    app.setBindAddress(modelData.ip)
                                }
                            }
                        }
                    }
                }
            }

            TextSwitch {
                width: parent.width
                text: qsTr("Web Server Active")
                description: bridge.web_server_running
                    ? qsTr("Serving documentation locally at %1").arg(bridge.web_server_url)
                    : qsTr("Start local web server to browse notes from desktop or another device")
                checked: bridge.web_server_running
                onCheckedChanged: {
                    if (checked && !bridge.web_server_running) {
                        bridge.start_web_server()
                    } else if (!checked && bridge.web_server_running) {
                        bridge.stop_web_server()
                    }
                }
            }

            Label {
                x: Theme.horizontalPageMargin
                width: parent.width - Theme.horizontalPageMargin * 2
                text: bridge.web_server_url
                color: Theme.highlightColor
                font.pixelSize: Theme.fontSizeSmall
                wrapMode: Text.WrapAnywhere
                visible: bridge.web_server_running
            }

            ComboBox {
                id: serverAddressCombo
                width: parent.width
                label: qsTr("Server Address")
                description: qsTr("Alternative addresses where the web server can be reached on your network")
                visible: bridge.web_server_running && networkInterfaces.length > 0
                menu: ContextMenu {
                    Repeater {
                        model: {
                            var list = []
                            for (var i = 0; i < networkInterfaces.length; i++) {
                                var iface = networkInterfaces[i]
                                var scheme = bridge.web_server_is_tls ? "https://" : "http://"
                                var port = bridge.web_server_port || 8080
                                list.push({
                                    name: iface.name,
                                    ip: iface.ip,
                                    url: scheme + iface.ip + ":" + port
                                })
                            }
                            return list
                        }
                        MenuItem {
                            text: modelData.name + ": " + modelData.url
                            onClicked: {
                                Clipboard.text = modelData.url
                                webRemorsePopup.execute(qsTr("Copied to clipboard: %1").arg(modelData.url), function() {})
                            }
                        }
                    }
                }
            }

            Button {
                anchors.horizontalCenter: parent.horizontalCenter
                text: qsTr("Copy Server URL")
                visible: bridge.web_server_running
                onClicked: {
                    Clipboard.text = bridge.web_server_url
                    webRemorsePopup.execute(qsTr("Copied to clipboard"), function() {})
                }
            }

            SectionHeader {
                text: qsTr("Web Server Authentication")
            }

            ComboBox {
                width: parent.width
                label: qsTr("Session Expiry")
                description: qsTr("How long browser login sessions stay valid before requiring re-approval")
                currentIndex: {
                    var hours = (typeof app !== "undefined" && app && app.sessionExpiryHours !== undefined) ? app.sessionExpiryHours : 24
                    switch (hours) {
                        case 1: return 0
                        case 8: return 1
                        case 24: return 2
                        case 72: return 3
                        case 168: return 4
                        case 0: return 5
                        default: return 2
                    }
                }
                menu: ContextMenu {
                    MenuItem {
                        text: qsTr("1 hour")
                        onClicked: if (typeof app !== "undefined" && app && app.setSessionExpiryHours) app.setSessionExpiryHours(1)
                    }
                    MenuItem {
                        text: qsTr("8 hours")
                        onClicked: if (typeof app !== "undefined" && app && app.setSessionExpiryHours) app.setSessionExpiryHours(8)
                    }
                    MenuItem {
                        text: qsTr("24 hours (Default)")
                        onClicked: if (typeof app !== "undefined" && app && app.setSessionExpiryHours) app.setSessionExpiryHours(24)
                    }
                    MenuItem {
                        text: qsTr("3 days")
                        onClicked: if (typeof app !== "undefined" && app && app.setSessionExpiryHours) app.setSessionExpiryHours(72)
                    }
                    MenuItem {
                        text: qsTr("7 days")
                        onClicked: if (typeof app !== "undefined" && app && app.setSessionExpiryHours) app.setSessionExpiryHours(168)
                    }
                    MenuItem {
                        text: qsTr("Never expire")
                        onClicked: if (typeof app !== "undefined" && app && app.setSessionExpiryHours) app.setSessionExpiryHours(0)
                    }
                }
            }

            Button {
                anchors.horizontalCenter: parent.horizontalCenter
                text: qsTr("Test Web Login Prompt")
                onClicked: {
                    pageStack.push(Qt.resolvedUrl("AuthPrompt.qml"), {
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
                            webRemorsePopup.execute("Error: " + err, function() {})
                        } else {
                            webRemorsePopup.execute("Installed SSL certificate", function() {})
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
                            webRemorsePopup.execute("Error: " + err, function() {})
                        } else {
                            webRemorsePopup.execute("Reset to self-signed certificate", function() {})
                        }
                    }
                }
            }
        }
    }
}
