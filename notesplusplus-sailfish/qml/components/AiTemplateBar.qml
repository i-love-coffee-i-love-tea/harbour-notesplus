import QtQuick 2.0
import Sailfish.Silica 1.0

Rectangle {
    id: templateBar
    width: parent.width - Theme.horizontalPageMargin * 2
    anchors.horizontalCenter: parent.horizontalCenter
    height: contentColumn.height + Theme.paddingMedium * 2
    radius: Theme.paddingSmall

    property bool enabled: true
    property bool agentBusy: false
    property bool hasContextOrInput: true
    property bool expanded: false

    readonly property var instructionsList: (typeof app !== "undefined" && app.customAiInstructions) ? app.customAiInstructions : []
    readonly property int totalCount: instructionsList.length

    color: Theme.rgba(Theme.highlightBackgroundColor, 0.08)
    border.color: Theme.rgba(Theme.primaryColor, 0.2)
    border.width: 1

    signal instructionSelected(var instructionItem)
    signal editInstructionRequested(var instructionItem)
    signal templateSelected(string templateName)

    Column {
        id: contentColumn
        anchors {
            left: parent.left
            right: parent.right
            top: parent.top
            margins: Theme.paddingMedium
        }
        spacing: Theme.paddingSmall

        // Header with Title, Count, Quick Actions, and Collapse/Expand Toggle
        BackgroundItem {
            id: headerItem
            width: parent.width
            height: Theme.itemSizeExtraSmall
            highlightedColor: "transparent"

            Row {
                anchors {
                    left: parent.left
                    verticalCenter: parent.verticalCenter
                }
                spacing: Theme.paddingSmall

                Icon {
                    source: "image://theme/icon-m-developer-mode"
                    width: Theme.iconSizeSmall
                    height: Theme.iconSizeSmall
                    color: headerItem.highlighted ? Theme.highlightColor : Theme.primaryColor
                    anchors.verticalCenter: parent.verticalCenter
                }

                Label {
                    text: qsTr("Quick Instructions")
                    font.bold: true
                    font.pixelSize: Theme.fontSizeSmall
                    color: headerItem.highlighted ? Theme.highlightColor : Theme.primaryColor
                    anchors.verticalCenter: parent.verticalCenter
                }

                Label {
                    text: "(%1)".arg(templateBar.totalCount)
                    font.pixelSize: Theme.fontSizeExtraSmall
                    color: Theme.secondaryColor
                    anchors.verticalCenter: parent.verticalCenter
                }
            }

            IconButton {
                anchors {
                    right: parent.right
                    verticalCenter: parent.verticalCenter
                }
                icon.source: templateBar.expanded ? "image://theme/icon-m-up" : "image://theme/icon-m-down"
                icon.width: Theme.iconSizeSmall
                icon.height: Theme.iconSizeSmall
                icon.color: headerItem.highlighted ? Theme.highlightColor : Theme.primaryColor
                width: Theme.itemSizeExtraSmall
                height: Theme.itemSizeExtraSmall
                onClicked: templateBar.expanded = !templateBar.expanded
            }

            onClicked: {
                templateBar.expanded = !templateBar.expanded
            }
        }

        // Instruction Buttons Grid (Responsive 2/3 column wrap, No Horizontal Scroll!)
        Grid {
            id: instructionsGrid
            width: parent.width
            visible: templateBar.expanded
            columns: width > 600 ? 3 : 2
            spacing: Theme.paddingSmall

            readonly property real itemWidth: Math.floor((width - (columns - 1) * spacing) / columns)

            Repeater {
                model: templateBar.instructionsList

                delegate: BackgroundItem {
                    id: tplBtn
                    width: instructionsGrid.itemWidth
                    height: Theme.itemSizeExtraSmall
                    enabled: templateBar.enabled && !templateBar.agentBusy
                    opacity: (templateBar.enabled && !templateBar.agentBusy) ? 1.0 : 0.4

                    readonly property bool isVendored: (typeof app !== "undefined" && app.isDefaultAiInstruction) ?
                                                           app.isDefaultAiInstruction(modelData.id) : false

                    Rectangle {
                        anchors.fill: parent
                        radius: Theme.paddingSmall / 2
                        color: tplBtn.highlighted ? Theme.rgba(Theme.highlightBackgroundColor, 0.45) :
                               (tplBtn.isVendored ? Theme.rgba(Theme.highlightBackgroundColor, 0.16) : Theme.rgba(Theme.primaryColor, 0.05))
                        border.color: tplBtn.highlighted ? Theme.highlightColor :
                                      (tplBtn.isVendored ? Theme.rgba(Theme.highlightColor, 0.35) : Theme.rgba(Theme.primaryColor, 0.2))
                        border.width: 1

                        Row {
                            anchors {
                                left: parent.left
                                right: parent.right
                                verticalCenter: parent.verticalCenter
                                margins: Theme.paddingSmall
                            }
                            spacing: Theme.paddingSmall

                            Icon {
                                source: modelData.icon ? (modelData.icon.indexOf("image://") === 0 ? modelData.icon : ("image://theme/" + modelData.icon)) : "image://theme/icon-m-note"
                                width: Theme.iconSizeSmall
                                height: Theme.iconSizeSmall
                                color: tplBtn.highlighted ? Theme.highlightColor :
                                       (tplBtn.isVendored ? Theme.primaryColor : Theme.secondaryColor)
                                anchors.verticalCenter: parent.verticalCenter
                            }

                            Label {
                                text: modelData.buttonText || modelData.title || ""
                                font.pixelSize: Theme.fontSizeExtraSmall
                                font.bold: true
                                color: tplBtn.highlighted ? Theme.highlightColor : Theme.primaryColor
                                anchors.verticalCenter: parent.verticalCenter
                                width: parent.width - Theme.iconSizeSmall - Theme.paddingSmall
                                truncationMode: TruncationMode.Fade
                            }
                        }
                    }

                    onClicked: {
                        templateBar.instructionSelected(modelData)
                        if (modelData.id) {
                            templateBar.templateSelected(modelData.id)
                        }
                    }

                    onPressAndHold: {
                        templateBar.editInstructionRequested(modelData)
                    }
                }
            }
        }

        // Context / Tip Hint
        Label {
            width: parent.width
            text: (!templateBar.hasContextOrInput) ?
                      qsTr("Tip: Tap to run (attach note if needed), hold to view or edit prompt.") :
                      qsTr("Tip: Tap to run action, hold to view or edit prompt.")
            font.pixelSize: Theme.fontSizeExtraSmall - 2
            color: Theme.secondaryColor
            wrapMode: Text.Wrap
            visible: templateBar.expanded
        }
    }
}
