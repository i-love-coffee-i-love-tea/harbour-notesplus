import QtQuick 2.6
import Sailfish.Silica 1.0
import "../../js/ThemeColors.js" as TC

Column {
    id: diffRoot
    width: parent.width
    spacing: Theme.paddingSmall

    property var diffData: null

    Row {
        width: parent.width
        spacing: Theme.paddingMedium
        visible: diffData !== null

        Label {
            text: diffData ? ("+" + (diffData.additions || 0) + " / -" + (diffData.deletions || 0) + " lines") : ""
            font.pixelSize: Theme.fontSizeExtraSmall
            color: Theme.secondaryColor
        }
    }

    Rectangle {
        width: parent.width
        height: diffColumn.height + Theme.paddingMedium * 2
        color: Theme.rgba(Theme.highlightBackgroundColor, 0.15)
        radius: Theme.paddingSmall
        clip: true

        Column {
            id: diffColumn
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: parent.top
            anchors.margins: Theme.paddingSmall
            spacing: 2

            Repeater {
                model: (diffData && diffData.lines) ? diffData.lines : []

                Rectangle {
                    width: diffColumn.width
                    height: lineContent.height + 4
                    color: modelData.line_type === "Add" ? "#264cd964" : (modelData.line_type === "Remove" ? "#26ff3b30" : "transparent")
                    radius: 2

                    Row {
                        id: lineContent
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.verticalCenter: parent.verticalCenter
                        anchors.leftMargin: Theme.paddingSmall
                        anchors.rightMargin: Theme.paddingSmall
                        spacing: Theme.paddingSmall

                        Label {
                            width: Theme.paddingLarge
                            text: modelData.line_type === "Add" ? "+" : (modelData.line_type === "Remove" ? "-" : " ")
                            font.family: "Monospace"
                            font.pixelSize: Theme.fontSizeExtraSmall
                            font.bold: true
                            color: modelData.line_type === "Add" ? TC.kDiffAdd : (modelData.line_type === "Remove" ? TC.kDiffRemove : Theme.secondaryColor)
                        }

                        Label {
                            width: parent.width - Theme.paddingLarge - Theme.paddingSmall
                            text: modelData.content || ""
                            font.family: "Monospace"
                            font.pixelSize: Theme.fontSizeExtraSmall
                            font.strikeout: modelData.line_type === "Remove"
                            color: modelData.line_type === "Add" ? TC.kDiffAdd : (modelData.line_type === "Remove" ? TC.kDiffRemove : Theme.primaryColor)
                            wrapMode: Text.WrapAnywhere
                        }
                    }
                }
            }
        }
    }
}
