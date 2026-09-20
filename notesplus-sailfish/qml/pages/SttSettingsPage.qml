import QtQuick 2.6
import Sailfish.Silica 1.0
import harbour.notesplus 1.0
import "../components/settings"

Page {
    id: sttSettingsPage
    allowedOrientations: Orientation.All

    property var sttModelsList: []

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

    Connections {
        target: (typeof speechBridge !== "undefined" && speechBridge) ? speechBridge : null
        onModels_changed: refreshSttModels()
        onDownloading_changed: refreshSttModels()
        onActive_model_changed: refreshSttModels()
        onDownload_completed: refreshSttModels()
    }

    Component.onCompleted: {
        if (typeof speechBridge !== "undefined" && speechBridge) {
            speechBridge.refresh_models()
        }
        refreshSttModels()
    }

    SilicaFlickable {
        anchors.fill: parent
        contentHeight: column.height + Theme.paddingLarge

        Column {
            id: column
            width: parent.width
            spacing: Theme.paddingMedium

            PageHeader {
                title: qsTr("Speech Recognition")
            }

            SttSettingsTab {
                page: sttSettingsPage
            }
        }
    }
}
