import QtQuick 2.6
import Sailfish.Silica 1.0
import harbour.notesplus 1.0
import "../components"

Dialog {
    id: modelDownloadDialog
    allowedOrientations: Orientation.All

    property var modelsList: []

    function refreshModelsList() {
        if (typeof speechBridge !== "undefined" && speechBridge && speechBridge.available_models_json) {
            try {
                modelsList = JSON.parse(speechBridge.available_models_json) || []
            } catch (e) {
                modelsList = []
            }
        } else {
            modelsList = []
        }
    }

    Component.onCompleted: {
        if (typeof speechBridge !== "undefined" && speechBridge) {
            speechBridge.refresh_models()
        }
        refreshModelsList()
    }

    Connections {
        target: (typeof speechBridge !== "undefined" && speechBridge) ? speechBridge : null
        onModels_changed: refreshModelsList()
        onDownloading_changed: refreshModelsList()
        onActive_model_changed: refreshModelsList()
        onDownload_completed: {
            refreshModelsList()
            var mId = (typeof model_id !== "undefined" && model_id) ? model_id :
                      ((typeof speechBridge !== "undefined" && speechBridge && speechBridge.active_model_id) ? speechBridge.active_model_id : "")
            if (typeof app !== "undefined" && app && app.setSttModel && mId) {
                app.setSttModel(mId)
            }
        }
        onError_occurred: {
            var errMsg = (typeof message !== "undefined" && message) ? message :
                         ((typeof speechBridge !== "undefined" && speechBridge && speechBridge.error_message) ? speechBridge.error_message : "")
            if (errMsg && errMsg.length > 0) {
                remorsePopup.execute(errMsg, function() {})
            }
        }
    }

    SilicaFlickable {
        anchors.fill: parent
        contentHeight: contentColumn.height + Theme.paddingLarge

        Column {
            id: contentColumn
            width: parent.width
            spacing: Theme.paddingMedium

            DialogHeader {
                title: qsTr("Speech Models")
                acceptText: qsTr("Done")
                cancelText: qsTr("Close")
            }

            // Info Card
            InfoCard {
                Label {
                    width: parent.width
                    text: qsTr("Offline Speech Recognition")
                    color: Theme.highlightColor
                    font.bold: true
                    font.pixelSize: Theme.fontSizeMedium
                }

                Label {
                    width: parent.width
                    text: qsTr("Speech recognition runs 100% offline on your device using Whisper models. Downloaded models are stored locally. Whisper Tiny (~75 MB) or Base (~142 MB) are recommended for fast performance.")
                    color: Theme.primaryColor
                    font.pixelSize: Theme.fontSizeExtraSmall
                    wrapMode: Text.Wrap
                }
            }

            SectionHeader {
                text: qsTr("Available Whisper Models")
            }

            // Model List
            Repeater {
                model: modelsList

                delegate: SttModelDelegate {
                    onDeleteRequested: function(modelId, modelName) {
                        if (typeof speechBridge !== "undefined" && speechBridge) {
                            speechBridge.delete_model(modelId)
                            remorsePopup.execute(qsTr("Deleted %1").arg(modelName), function() {})
                        }
                    }
                }
            }

            // Empty state if no models in catalog
            Label {
                visible: modelsList.length === 0
                width: parent.width - Theme.horizontalPageMargin * 2
                anchors.horizontalCenter: parent.horizontalCenter
                horizontalAlignment: Text.AlignHCenter
                text: qsTr("No speech models available.")
                color: Theme.secondaryColor
                font.pixelSize: Theme.fontSizeSmall
            }
        }
    }

    RemorsePopup {
        id: remorsePopup
    }
}
