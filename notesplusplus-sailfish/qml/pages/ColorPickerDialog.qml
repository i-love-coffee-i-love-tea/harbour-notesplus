import QtQuick 2.6
import Sailfish.Silica 1.0
import "../js/ThemeColors.js" as TC

Dialog {
    id: colorPickerDialog
    allowedOrientations: Orientation.All

    property string selectedColor: ""

    canAccept: true

    Column {
        width: parent.width
        spacing: Theme.paddingMedium

        DialogHeader {
            title: qsTr("Select Note Color")
            acceptText: qsTr("Select")
            cancelText: qsTr("Cancel")
        }

        Item {
            width: parent.width
            height: Theme.paddingSmall
        }

        Rectangle {
            anchors.horizontalCenter: parent.horizontalCenter
            width: Theme.itemSizeMedium
            height: Theme.itemSizeExtraSmall
            radius: Theme.paddingSmall
            color: selectedColor.length > 0 ? selectedColor : Theme.highlightColor
            border.width: 1
            border.color: Theme.rgba(Theme.primaryColor, 0.3)
        }

        Button {
            anchors.horizontalCenter: parent.horizontalCenter
            text: qsTr("Default Color")
            preferredWidth: Theme.buttonWidthMedium
            onClicked: {
                colorPickerDialog.selectedColor = ""
                colorPickerDialog.accept()
            }
        }

        SectionHeader {
            text: qsTr("Palette")
        }

        Grid {
            columns: 4
            spacing: Theme.paddingLarge
            anchors.horizontalCenter: parent.horizontalCenter

            Repeater {
                model: TC.kNoteCardPalette
                Rectangle {
                    width: Theme.itemSizeExtraSmall
                    height: width
                    radius: width / 2
                    color: modelData
                    border.width: colorPickerDialog.selectedColor === modelData ? 3 : 1
                    border.color: colorPickerDialog.selectedColor === modelData ? Theme.highlightColor : Theme.rgba(Theme.primaryColor, 0.2)

                    Rectangle {
                        anchors.centerIn: parent
                        width: parent.width * 0.4
                        height: width
                        radius: width / 2
                        color: Theme.highlightColor
                        visible: colorPickerDialog.selectedColor === modelData
                    }

                    MouseArea {
                        anchors.fill: parent
                        onClicked: {
                            colorPickerDialog.selectedColor = modelData
                        }
                    }
                }
            }
        }
    }
}
