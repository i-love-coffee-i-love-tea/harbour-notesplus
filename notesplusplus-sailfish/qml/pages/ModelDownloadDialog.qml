import QtQuick 2.6
import Sailfish.Silica 1.0
import harbour.notesplusplus 1.0

Dialog {
    id: modelDownloadDialog
    allowedOrientations: Orientation.All

    property var modelsList: []

    function formatSize(bytes) {
        if (!bytes || bytes <= 0) return ""
        var mb = bytes / (1024 * 1024)
        return mb.toFixed(0) + " MB"
    }

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

    Timer {
        id: pollTimer
        interval: 100
        running: typeof speechBridge !== "undefined" && speechBridge && (speechBridge.is_downloading || speechBridge.is_transcribing)
        repeat: true
        onTriggered: {
            if (typeof speechBridge !== "undefined" && speechBridge) {
                speechBridge.poll_worker()
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
            Rectangle {
                width: parent.width - Theme.horizontalPageMargin * 2
                anchors.horizontalCenter: parent.horizontalCenter
                height: infoCol.height + Theme.paddingMedium * 2
                color: Theme.rgba(Theme.highlightBackgroundColor, 0.1)
                border.color: Theme.rgba(Theme.highlightColor, 0.3)
                border.width: 1
                radius: Theme.paddingSmall

                Column {
                    id: infoCol
                    anchors {
                        left: parent.left
                        right: parent.right
                        top: parent.top
                        margins: Theme.paddingMedium
                    }
                    spacing: Theme.paddingSmall

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
            }

            SectionHeader {
                text: qsTr("Available Whisper Models")
            }

            // Model List
            Repeater {
                model: modelsList

                delegate: BackgroundItem {
                    id: modelDelegate
                    width: parent.width
                    height: itemColumn.height + Theme.paddingMedium * 2

                    readonly property var modelItem: modelData
                    readonly property bool isCurrentDownloading: typeof speechBridge !== "undefined" && speechBridge && speechBridge.is_downloading && speechBridge.downloading_model_id === modelItem.id
                    readonly property bool isModelActive: (typeof speechBridge !== "undefined" && speechBridge && speechBridge.active_model_id === modelItem.id) || (modelItem.is_active === true)
                    readonly property bool isModelInstalled: modelItem.is_installed === true

                    Column {
                        id: itemColumn
                        anchors {
                            left: parent.left
                            right: parent.right
                            top: parent.top
                            leftMargin: Theme.horizontalPageMargin
                            rightMargin: Theme.horizontalPageMargin
                            topMargin: Theme.paddingSmall
                        }
                        spacing: Theme.paddingSmall

                        // Header row with Name and Badges
                        Row {
                            width: parent.width
                            spacing: Theme.paddingSmall

                            Label {
                                text: modelItem.name || modelItem.id
                                color: isModelActive ? Theme.highlightColor : Theme.primaryColor
                                font.bold: true
                                font.pixelSize: Theme.fontSizeMedium
                            }

                            Rectangle {
                                visible: isModelActive
                                anchors.verticalCenter: parent.verticalCenter
                                width: activeLabel.width + Theme.paddingSmall
                                height: activeLabel.height + Theme.paddingSmall / 2
                                color: Theme.rgba(Theme.highlightColor, 0.25)
                                border.color: Theme.highlightColor
                                border.width: 1
                                radius: 4

                                Label {
                                    id: activeLabel
                                    anchors.centerIn: parent
                                    text: qsTr("ACTIVE")
                                    color: Theme.highlightColor
                                    font.bold: true
                                    font.pixelSize: Theme.fontSizeTiny
                                }
                            }
                        }

                        // Meta row: Language & Size
                        Row {
                            width: parent.width
                            spacing: Theme.paddingMedium

                            Label {
                                text: modelItem.is_multilingual ? qsTr("Multilingual") : qsTr("English only")
                                color: Theme.secondaryColor
                                font.pixelSize: Theme.fontSizeExtraSmall
                            }

                            Label {
                                text: formatSize(modelItem.size_bytes)
                                color: Theme.secondaryColor
                                font.pixelSize: Theme.fontSizeExtraSmall
                            }

                            Label {
                                text: isModelInstalled ? qsTr("Installed") : qsTr("Not downloaded")
                                color: isModelInstalled ? Theme.highlightColor : Theme.secondaryColor
                                font.pixelSize: Theme.fontSizeExtraSmall
                            }
                        }

                        // Description
                        Label {
                            width: parent.width
                            text: modelItem.description || ""
                            color: Theme.secondaryColor
                            font.pixelSize: Theme.fontSizeSmall
                            wrapMode: Text.Wrap
                        }

                        // Active Download Progress
                        Column {
                            width: parent.width
                            visible: isCurrentDownloading
                            spacing: Theme.paddingSmall

                            ProgressBar {
                                width: parent.width
                                minimumValue: 0
                                maximumValue: 100
                                value: typeof speechBridge !== "undefined" && speechBridge ? speechBridge.download_progress : 0
                                label: qsTr("Downloading model...")
                                valueText: Math.round(value) + "%"
                            }

                            Button {
                                anchors.horizontalCenter: parent.horizontalCenter
                                text: qsTr("Cancel Download")
                                preferredWidth: Theme.buttonWidthSmall
                                onClicked: {
                                    if (typeof speechBridge !== "undefined" && speechBridge) {
                                        speechBridge.cancel_download()
                                    }
                                }
                            }
                        }

                        // Installed Actions
                        Row {
                            visible: isModelInstalled && !isCurrentDownloading
                            spacing: Theme.paddingMedium
                            anchors.horizontalCenter: parent.horizontalCenter

                            Button {
                                text: qsTr("Set Active")
                                visible: !isModelActive
                                preferredWidth: Theme.buttonWidthSmall
                                onClicked: {
                                    if (typeof speechBridge !== "undefined" && speechBridge) {
                                        speechBridge.set_active_model(modelItem.id)
                                    }
                                    if (typeof app !== "undefined" && app && app.setSttModel) {
                                        app.setSttModel(modelItem.id)
                                    }
                                }
                            }

                            Button {
                                text: qsTr("Delete")
                                preferredWidth: Theme.buttonWidthSmall
                                color: Theme.highlightColor
                                onClicked: {
                                    if (typeof speechBridge !== "undefined" && speechBridge) {
                                        speechBridge.delete_model(modelItem.id)
                                        remorsePopup.execute(qsTr("Deleted %1").arg(modelItem.name), function() {})
                                    }
                                }
                            }
                        }

                        // Not Installed Actions
                        Row {
                            visible: !isModelInstalled && !isCurrentDownloading
                            anchors.horizontalCenter: parent.horizontalCenter

                            Button {
                                text: qsTr("Download (%1)").arg(formatSize(modelItem.size_bytes))
                                preferredWidth: Theme.buttonWidthMedium
                                enabled: !(typeof speechBridge !== "undefined" && speechBridge && speechBridge.is_downloading)
                                onClicked: {
                                    if (typeof speechBridge !== "undefined" && speechBridge) {
                                        speechBridge.download_model(modelItem.id)
                                    }
                                }
                            }
                        }

                        // Bottom separator
                        Separator {
                            width: parent.width
                            color: Theme.rgba(Theme.primaryColor, 0.1)
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
