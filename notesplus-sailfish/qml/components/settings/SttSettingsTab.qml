import QtQuick 2.6
import Sailfish.Silica 1.0
import harbour.notesplus 1.0
import "."
import "../common"

Column {
    id: sttSettingsTab
    width: parent.width
    spacing: Theme.paddingMedium

    // Properties passed from SettingsPage
    property var page: null  // reference to SettingsPage for shared state

    RemorsePopup {
        id: sttRemorsePopup
    }

    SectionHeader {
        text: qsTr("Speech Recognition (Offline STT)")
    }

    TextSwitch {
        width: parent.width
        text: qsTr("Enable Speech Recognition")
        description: qsTr("Dictate prompts and transcribe speech offline using on-device Whisper models")
        checked: (typeof app !== "undefined" && app && app.sttEnabled !== undefined) ? app.sttEnabled : true
        onCheckedChanged: {
            if (typeof app !== "undefined" && app && app.setSttEnabled) {
                app.setSttEnabled(checked)
            }
        }
    }

    Column {
        width: parent.width
        spacing: Theme.paddingMedium
        visible: (typeof app !== "undefined" && app && app.sttEnabled !== undefined) ? app.sttEnabled : true

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
            model: page.sttModelsList

            delegate: SttModelDelegate {
                onDeleteRequested: function(modelId, modelName) {
                    if (typeof speechBridge !== "undefined" && speechBridge) {
                        speechBridge.delete_model(modelId)
                        sttRemorsePopup.execute(qsTr("Deleted %1").arg(modelName), function() {})
                    }
                }
            }
        }

        // Empty state if no models in catalog
        Label {
            visible: page.sttModelsList.length === 0
            width: parent.width - Theme.horizontalPageMargin * 2
            anchors.horizontalCenter: parent.horizontalCenter
            horizontalAlignment: Text.AlignHCenter
            text: qsTr("No speech models available.")
            color: Theme.secondaryColor
            font.pixelSize: Theme.fontSizeSmall
        }
    }
}
