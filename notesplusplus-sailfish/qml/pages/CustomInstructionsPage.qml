import QtQuick 2.0
import Sailfish.Silica 1.0

Page {
    id: customInstructionsPage
    allowedOrientations: Orientation.All

    SilicaListView {
        id: listView
        anchors.fill: parent
        model: typeof app !== "undefined" ? app.customAiInstructions : []

        PullDownMenu {
            MenuItem {
                text: qsTr("Reset to Defaults")
                onClicked: {
                    Remorse.popupAction(customInstructionsPage, qsTr("Resetting AI instructions"), function() {
                        if (typeof app !== "undefined" && app.resetCustomAiInstructions) {
                            app.resetCustomAiInstructions()
                        }
                    })
                }
            }

            MenuItem {
                text: qsTr("Add AI Instruction")
                onClicked: {
                    pageStack.push(Qt.resolvedUrl("CustomInstructionDialog.qml"))
                }
            }
        }

        header: Column {
            width: parent.width
            spacing: Theme.paddingMedium

            PageHeader {
                title: qsTr("AI Instructions")
            }

            Label {
                width: parent.width - Theme.horizontalPageMargin * 2
                anchors.horizontalCenter: parent.horizontalCenter
                text: qsTr("Customize prompt buttons displayed above the AI chat input. Each instruction can have its own button label, icon, and custom prompt.")
                font.pixelSize: Theme.fontSizeExtraSmall
                color: Theme.secondaryColor
                wrapMode: Text.Wrap
            }

            Button {
                anchors.horizontalCenter: parent.horizontalCenter
                text: qsTr("+ Add AI Instruction")
                onClicked: pageStack.push(Qt.resolvedUrl("CustomInstructionDialog.qml"))
            }

            SectionHeader {
                text: qsTr("Configured Instructions (%1)").arg(listView.count)
            }
        }

        delegate: ListItem {
            id: instructionItem
            contentHeight: Theme.itemSizeMedium

            function edit() {
                var inst = modelData.instruction || ""
                if (!inst && typeof app !== "undefined" && app.defaultCustomAiInstructions) {
                    for (var k = 0; k < app.defaultCustomAiInstructions.length; k++) {
                        if (app.defaultCustomAiInstructions[k].id === modelData.id) {
                            inst = app.defaultCustomAiInstructions[k].instruction || ""
                            break
                        }
                    }
                }
                pageStack.push(Qt.resolvedUrl("CustomInstructionDialog.qml"), {
                    "instructionId": modelData.id || "",
                    "initialButtonText": modelData.buttonText || "",
                    "initialIcon": modelData.icon || "icon-m-note",
                    "initialInstruction": inst,
                    "isEdit": true
                })
            }

            function remove() {
                remorseAction(qsTr("Deleting instruction"), function() {
                    if (typeof app !== "undefined" && app.deleteCustomAiInstruction) {
                        app.deleteCustomAiInstruction(modelData.id)
                    }
                })
            }

            menu: ContextMenu {
                MenuItem {
                    text: qsTr("Edit")
                    onClicked: instructionItem.edit()
                }

                MenuItem {
                    text: qsTr("Delete")
                    onClicked: instructionItem.remove()
                }
            }

            onClicked: instructionItem.edit()

            Row {
                anchors {
                    left: parent.left
                    right: parent.right
                    verticalCenter: parent.verticalCenter
                    margins: Theme.horizontalPageMargin
                }
                spacing: Theme.paddingMedium

                Rectangle {
                    width: Theme.itemSizeExtraSmall
                    height: Theme.itemSizeExtraSmall
                    radius: Theme.paddingSmall / 2
                    color: Theme.rgba(Theme.highlightBackgroundColor, 0.2)
                    border.color: Theme.rgba(Theme.highlightColor, 0.4)
                    border.width: 1
                    anchors.verticalCenter: parent.verticalCenter

                    Icon {
                        anchors.centerIn: parent
                        source: modelData.icon ? (modelData.icon.indexOf("image://") === 0 ? modelData.icon : ("image://theme/" + modelData.icon)) : "image://theme/icon-m-note"
                        width: Theme.iconSizeMedium
                        height: Theme.iconSizeMedium
                        color: Theme.highlightColor
                    }
                }

                Column {
                    width: parent.width - Theme.itemSizeExtraSmall - Theme.paddingMedium
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: 2

                    Label {
                        text: modelData.buttonText || qsTr("Untitled Instruction")
                        font.pixelSize: Theme.fontSizeMedium
                        font.bold: true
                        color: instructionItem.highlighted ? Theme.highlightColor : Theme.primaryColor
                        truncationMode: TruncationMode.Fade
                        width: parent.width
                    }

                    Label {
                        text: modelData.instruction || ""
                        font.pixelSize: Theme.fontSizeExtraSmall
                        color: Theme.secondaryColor
                        truncationMode: TruncationMode.Fade
                        width: parent.width
                    }
                }
            }
        }

        ViewPlaceholder {
            enabled: listView.count === 0
            text: qsTr("No AI instructions configured")
            hintText: qsTr("Pull down or tap '+ Add AI Instruction' to create one.")
        }
    }
}
