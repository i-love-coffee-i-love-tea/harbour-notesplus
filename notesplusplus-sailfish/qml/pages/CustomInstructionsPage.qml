import QtQuick 2.6
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
                text: qsTr("Reset Built-in Defaults")
                onClicked: {
                    Remorse.popupAction(customInstructionsPage, qsTr("Resetting built-in instructions to defaults"), function() {
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
            contentHeight: Math.max(Theme.itemSizeMedium, itemRow.height + Theme.paddingMedium * 2)

            readonly property bool isVendored: (typeof app !== "undefined" && app.isDefaultAiInstruction) ?
                                                   app.isDefaultAiInstruction(modelData.id) : false

            readonly property string fullInstructionText: {
                if (modelData.instruction && modelData.instruction.length > 0) {
                    return modelData.instruction
                }
                if (typeof app !== "undefined" && app.getDefaultAiInstruction) {
                    var def = app.getDefaultAiInstruction(modelData.id)
                    if (def && def.instruction) {
                        return def.instruction
                    }
                }
                return ""
            }

            function edit() {
                pageStack.push(Qt.resolvedUrl("CustomInstructionDialog.qml"), {
                    "instructionId": modelData.id || "",
                    "initialButtonText": modelData.buttonText || "",
                    "initialIcon": modelData.icon || "icon-m-note",
                    "initialInstruction": instructionItem.fullInstructionText,
                    "isEdit": true
                })
            }

            function resetToDefault() {
                remorseAction(qsTr("Resetting instruction to default"), function() {
                    if (typeof app !== "undefined" && app.resetSingleAiInstruction) {
                        app.resetSingleAiInstruction(modelData.id)
                    }
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
                    text: qsTr("Reset to Default")
                    visible: instructionItem.isVendored
                    onClicked: instructionItem.resetToDefault()
                }

                MenuItem {
                    text: qsTr("Delete")
                    onClicked: instructionItem.remove()
                }
            }

            onClicked: instructionItem.edit()

            Row {
                id: itemRow
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
                    color: instructionItem.isVendored ?
                               Theme.rgba(Theme.highlightBackgroundColor, 0.25) :
                               Theme.rgba(Theme.primaryColor, 0.08)
                    border.color: instructionItem.isVendored ?
                                      Theme.rgba(Theme.highlightColor, 0.35) :
                                      Theme.rgba(Theme.primaryColor, 0.2)
                    border.width: 1
                    anchors.verticalCenter: parent.verticalCenter

                    Icon {
                        anchors.centerIn: parent
                        source: modelData.icon ? (modelData.icon.indexOf("image://") === 0 ? modelData.icon : ("image://theme/" + modelData.icon)) : "image://theme/icon-m-note"
                        width: Theme.iconSizeMedium
                        height: Theme.iconSizeMedium
                        color: instructionItem.highlighted ? Theme.highlightColor :
                               (instructionItem.isVendored ? Theme.primaryColor : Theme.secondaryColor)
                    }
                }

                Column {
                    width: parent.width - Theme.itemSizeExtraSmall - Theme.paddingMedium
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: Theme.paddingSmall / 2

                    Row {
                        width: parent.width
                        spacing: Theme.paddingSmall

                        Label {
                            text: modelData.buttonText || qsTr("Untitled Instruction")
                            font.pixelSize: Theme.fontSizeMedium
                            font.bold: true
                            color: instructionItem.highlighted ? Theme.highlightColor : Theme.primaryColor
                            truncationMode: TruncationMode.Fade
                            anchors.verticalCenter: parent.verticalCenter
                            width: Math.min(implicitWidth, parent.width - badgeRect.width - Theme.paddingSmall)
                        }

                        Rectangle {
                            id: badgeRect
                            height: badgeLabel.height + 4
                            width: badgeLabel.width + Theme.paddingSmall
                            radius: 3
                            anchors.verticalCenter: parent.verticalCenter
                            color: instructionItem.isVendored ?
                                       Theme.rgba(Theme.highlightBackgroundColor, 0.3) :
                                       Theme.rgba(Theme.primaryColor, 0.1)
                            border.color: instructionItem.isVendored ?
                                              Theme.rgba(Theme.highlightColor, 0.4) :
                                              Theme.rgba(Theme.primaryColor, 0.2)
                            border.width: 1

                            Label {
                                id: badgeLabel
                                anchors.centerIn: parent
                                text: instructionItem.isVendored ? qsTr("Built-in") : qsTr("Custom")
                                font.pixelSize: Theme.fontSizeTiny
                                font.bold: true
                                color: instructionItem.highlighted ? Theme.highlightColor :
                                       (instructionItem.isVendored ? Theme.primaryColor : Theme.secondaryColor)
                            }
                        }
                    }

                    Label {
                        text: instructionItem.fullInstructionText
                        font.pixelSize: Theme.fontSizeExtraSmall
                        color: Theme.secondaryColor
                        wrapMode: Text.Wrap
                        maximumLineCount: 3
                        truncationMode: TruncationMode.Fade
                        width: parent.width
                        visible: text.length > 0
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
