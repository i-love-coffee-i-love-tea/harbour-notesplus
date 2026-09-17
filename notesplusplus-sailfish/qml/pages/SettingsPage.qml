import QtQuick 2.6
import Sailfish.Silica 1.0
import harbour.notesplus 1.0
import "../components"

Page {
    id: settingsPage
    allowedOrientations: Orientation.All

    property int currentTab: 0
    property var serverModels: []
    property bool modelsLoaded: false
    property var sttModelsList: []
    property var networkInterfaces: []

    function refreshSttModels() {
        if (typeof speechBridge !== "undefined" && speechBridge && speechBridge.available_models_json) {
            try {
                sttModelsList = JSON.parse(speechBridge.available_models_json) || []
            } catch (e) {
                sttModelsList = []
            }
        } else {
            sttModelsList = []
        }
    }

    function refreshNetworkInterfaces() {
        try {
            var json = bridge.get_network_interfaces_json()
            networkInterfaces = JSON.parse(json) || []
        } catch (e) {
            networkInterfaces = []
        }
    }
    property var displayModels: {
        var list = serverModels || []
        var current = (typeof app !== "undefined" && app.aiModel) ? app.aiModel : ""
        if (current.length > 0) {
            // Only flag "not on server" if the fetch actually succeeded
            if (modelsLoaded && modelError.length === 0) {
                var found = false
                for (var i = 0; i < list.length; i++) {
                    if ((list[i].id || list[i].name) === current) { found = true; break }
                }
                if (!found) {
                    return [{id: current, name: current + " (not on server)"}].concat(list)
                }
            }
            // Always ensure the configured model appears in the list
            if (list.length === 0) {
                return [{id: current, name: current}]
            }
        }
        return list
    }

    property string modelError: ""

    function refreshModels() {
        if (typeof app === "undefined" || !app) return
        console.log("[SettingsPage] refreshModels called, provider:", app.aiProvider, "endpoint:", app.aiEndpoint)
        modelError = ""
        modelBridge.configure(
            app.aiProvider === "mimocode" ? "mimocode" : "ollama",
            app.aiEndpoint,
            app.aiModel,
            app.aiApiKey || "",
            app.aiTimeout || 90,
            true, true, true,
            app.aiAllowSelfSigned || false,
            true
        )
        modelBridge.fetch_models()
    }

    AgentBridge {
        id: modelBridge
        onError_occurred: {
            console.log("[SettingsPage] modelBridge error_occurred:", message)
            settingsPage.modelError = message
        }
        onModels_changed: {
            console.log("[SettingsPage] models_changed, loading:", models_loading, "models:", available_models)
            if (!models_loading) {
                try {
                    var list = JSON.parse(available_models)
                    serverModels = list
                    modelsLoaded = true
                } catch (e) {
                    serverModels = []
                    modelsLoaded = true
                }
            }
        }
    }

    onCurrentTabChanged: {
        if (currentTab === 1 && !modelsLoaded) {
            refreshModels()
        }
        if (currentTab === 2) {
            if (typeof speechBridge !== "undefined" && speechBridge) {
                speechBridge.refresh_models()
            }
            refreshSttModels()
        }
        if (currentTab === 3 && networkInterfaces.length === 0) {
            refreshNetworkInterfaces()
        }
    }

    Connections {
        target: (typeof speechBridge !== "undefined" && speechBridge) ? speechBridge : null
        onModels_changed: refreshSttModels()
        onDownloading_changed: refreshSttModels()
        onActive_model_changed: refreshSttModels()
        onDownload_completed: refreshSttModels()
    }

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
                    preferredWidth: Math.floor((parent.width - Theme.paddingSmall * 3) / 4)
                    color: currentTab === 0 ? Theme.highlightColor : Theme.primaryColor
                    backgroundColor: currentTab === 0 ? Theme.rgba(Theme.highlightBackgroundColor, 0.4) : "transparent"
                    onClicked: currentTab = 0
                }

                Button {
                    text: qsTr("Assistant")
                    preferredWidth: Math.floor((parent.width - Theme.paddingSmall * 3) / 4)
                    color: currentTab === 1 ? Theme.highlightColor : Theme.primaryColor
                    backgroundColor: currentTab === 1 ? Theme.rgba(Theme.highlightBackgroundColor, 0.4) : "transparent"
                    onClicked: currentTab = 1
                }

                Button {
                    text: qsTr("STT")
                    preferredWidth: Math.floor((parent.width - Theme.paddingSmall * 3) / 4)
                    color: currentTab === 2 ? Theme.highlightColor : Theme.primaryColor
                    backgroundColor: currentTab === 2 ? Theme.rgba(Theme.highlightBackgroundColor, 0.4) : "transparent"
                    onClicked: currentTab = 2
                }

                Button {
                    text: qsTr("Services")
                    preferredWidth: parent.width - Theme.paddingSmall * 3 - Math.floor((parent.width - Theme.paddingSmall * 3) / 4) * 3
                    color: currentTab === 3 ? Theme.highlightColor : Theme.primaryColor
                    backgroundColor: currentTab === 3 ? Theme.rgba(Theme.highlightBackgroundColor, 0.4) : "transparent"
                    onClicked: currentTab = 3
                }
            }

            // Tab Content
            DisplaySettingsTab {
                visible: currentTab === 0
            }

            AssistantSettingsTab {
                visible: currentTab === 1
                page: settingsPage
                modelBridge: settingsPage.modelBridge
            }

            SttSettingsTab {
                visible: currentTab === 2
                page: settingsPage
            }

            ServicesSettingsTab {
                visible: currentTab === 3
                page: settingsPage
            }
        }
    }
}
