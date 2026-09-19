import QtQuick 2.6
import Sailfish.Silica 1.0

BackgroundItem {
    id: sttModelDelegate
    width: parent.width
    height: sttItemColumn.height + Theme.paddingMedium * 2

    readonly property var modelItem: modelData
    readonly property bool isCurrentDownloading: typeof speechBridge !== "undefined" && speechBridge && speechBridge.is_downloading && speechBridge.downloading_model_id === modelItem.id
    readonly property bool isModelActive: (typeof speechBridge !== "undefined" && speechBridge && speechBridge.active_model_id === modelItem.id) || (modelItem.is_active === true)
    readonly property bool isModelInstalled: modelItem.is_installed === true

    signal deleteRequested(string modelId, string modelName)

    Column {
        id: sttItemColumn
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
                width: sttActiveLabel.width + Theme.paddingSmall
                height: sttActiveLabel.height + Theme.paddingSmall / 2
                color: Theme.rgba(Theme.highlightColor, 0.25)
                border.color: Theme.highlightColor
                border.width: 1
                radius: 4

                Label {
                    id: sttActiveLabel
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
                text: app.formatSize(modelItem.size_bytes)
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
                    sttModelDelegate.deleteRequested(modelItem.id, modelItem.name || modelItem.id)
                }
            }
        }

        // Not Installed Actions
        Row {
            visible: !isModelInstalled && !isCurrentDownloading
            anchors.horizontalCenter: parent.horizontalCenter

            Button {
                text: qsTr("Download (%1)").arg(app.formatSize(modelItem.size_bytes))
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
