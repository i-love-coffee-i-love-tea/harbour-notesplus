import QtQuick 2.6
import Sailfish.Silica 1.0
import harbour.notesplus 1.0
import "../components/settings"

Page {
    id: assistantSettingsPage
    allowedOrientations: Orientation.All

    property var serverModels: []
    property bool modelsLoaded: false
    property string modelError: ""

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

    function refreshModels() {
        if (typeof app === "undefined" || !app) return
        console.log("[AssistantSettingsPage] refreshModels called, provider:", app.aiProvider, "endpoint:", app.aiEndpoint)
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
            console.log("[AssistantSettingsPage] modelBridge error_occurred:", message)
            assistantSettingsPage.modelError = message
        }
        onModels_changed: {
            console.log("[AssistantSettingsPage] models_changed, loading:", models_loading, "models:", available_models)
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

    Component.onCompleted: {
        refreshModels()
    }

    SilicaFlickable {
        anchors.fill: parent
        contentHeight: column.height + Theme.paddingLarge

        Column {
            id: column
            width: parent.width
            spacing: Theme.paddingMedium

            PageHeader {
                title: qsTr("AI Assistant")
            }

            AssistantSettingsTab {
                page: assistantSettingsPage
                modelBridge: assistantSettingsPage.modelBridge
            }
        }
    }
}
