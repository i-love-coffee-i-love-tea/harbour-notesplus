import QtQuick 2.6
import Sailfish.Silica 1.0
import "../"
import "../../js/BlockHtmlUtils.js" as BlockHtmlUtils

Column {
    id: codeDelegate
    property var blockData: ({})

    anchors.left: parent ? parent.left : undefined
    anchors.right: parent ? parent.right : undefined
    anchors.leftMargin: Theme.horizontalPageMargin
    anchors.rightMargin: Theme.horizontalPageMargin
    spacing: Theme.paddingSmall / 2
    clip: true

    InlineText {
        visible: Boolean(blockData && ((blockData.title_spans && blockData.title_spans.length > 0) || (blockData.title && blockData.title.length > 0)))
        spans: (blockData && blockData.title_spans && blockData.title_spans.length > 0) ? blockData.title_spans : ((blockData && blockData.title) ? [{ type: "text", value: blockData.title }] : undefined)
        font.family: (typeof app !== "undefined" && app && app.docFontFamily && app.docFontFamily.length > 0) ? app.docFontFamily : Theme.fontFamily
        font.pixelSize: Math.round(Theme.fontSizeExtraSmall * ((typeof app !== "undefined" && app && app.fontScale) ? app.fontScale : 1.0))
        font.bold: true
        color: Theme.highlightColor
        wrapMode: Text.WrapAtWordBoundaryOrAnywhere
        width: parent.width
    }

    Rectangle {
        color: "#18181c"
        border.color: Theme.rgba(Theme.primaryColor, 0.2)
        border.width: 1
        radius: 0
        height: codeCol.height + Theme.paddingMedium * 2
        anchors.left: parent.left
        anchors.right: parent.right
        clip: true

        Column {
            id: codeCol
            anchors {
                left: parent.left
                right: parent.right
                top: parent.top
                margins: Theme.paddingMedium
            }
            spacing: Theme.paddingSmall / 2

            Label {
                visible: (blockData.language || "").length > 0
                text: (blockData.language || "").toUpperCase()
                font.family: "monospace"
                font.pixelSize: Math.round(Theme.fontSizeExtraSmall * (typeof app !== "undefined" && app && app.codeFontScale ? app.codeFontScale : 1.0))
                color: Theme.rgba("#f2f2f7", 0.6)
            }

            Label {
                id: codeLabel
                anchors.left: parent.left
                anchors.right: parent.right
                text: (blockData.lines || []).join("\n")
                font.family: "monospace"
                font.pixelSize: Math.round(Theme.fontSizeSmall * (typeof app !== "undefined" && app && app.codeFontScale ? app.codeFontScale : 1.0))
                color: "#f2f2f7"
                wrapMode: Text.WrapAtWordBoundaryOrAnywhere
            }
        }
    }
}
