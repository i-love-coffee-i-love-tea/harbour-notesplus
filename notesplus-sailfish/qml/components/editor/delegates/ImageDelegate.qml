import QtQuick 2.6
import Sailfish.Silica 1.0
import "../"
import "../../common"
import "../../../js/BlockHtmlUtils.js" as BlockHtmlUtils

Column {
    id: imageDelegate
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
        font.family: app.resolvedFontFamily()
        font.pixelSize: app.scaledFontSize(Theme.fontSizeExtraSmall)
        font.bold: true
        color: Theme.highlightColor
        wrapMode: Text.WrapAtWordBoundaryOrAnywhere
        width: parent.width
    }

    Item {
        width: parent.width
        height: img.status === Image.Ready ? (img.height + (captionLabel.visible ? captionLabel.height + Theme.paddingSmall : 0) + Theme.paddingMedium) : (errorLabel.visible ? (errorLabel.height + Theme.paddingMedium) : Theme.itemSizeMedium)

        Image {
            id: img
            anchors.top: parent.top
            anchors.horizontalCenter: parent.horizontalCenter
            width: {
                if (blockData && blockData.width) {
                    var w = parseInt(blockData.width, 10)
                    if (!isNaN(w) && w > 0) return Math.min(w, parent.width)
                }
                return Math.min(implicitWidth > 0 ? implicitWidth : parent.width, parent.width)
            }
            fillMode: Image.PreserveAspectFit
            source: BlockHtmlUtils.resolveImagePath(blockData ? (blockData.target || "") : "", (typeof bridge !== "undefined" && bridge) ? bridge.notes_dir : "", (typeof app !== "undefined" && app) ? app.allowExternalImages : true)
            asynchronous: true
        }

        Label {
            id: captionLabel
            anchors.top: img.bottom
            anchors.topMargin: Theme.paddingSmall
            anchors.horizontalCenter: parent.horizontalCenter
            width: parent.width
            horizontalAlignment: Text.AlignHCenter
            font.pixelSize: Theme.fontSizeExtraSmall
            color: Theme.secondaryColor
            text: (blockData && blockData.alt) ? blockData.alt : ""
            visible: text.length > 0 && img.status === Image.Ready
            wrapMode: Text.Wrap
        }

        Label {
            id: errorLabel
            anchors.centerIn: parent
            width: parent.width
            horizontalAlignment: Text.AlignHCenter
            font.pixelSize: Theme.fontSizeSmall
            color: Theme.secondaryColor
            text: (blockData && blockData.alt) ? (blockData.alt + " (" + (blockData.target || "") + ")") : ((blockData && blockData.target) ? blockData.target : "Image")
            visible: img.status === Image.Error || img.status === Image.Null
            wrapMode: Text.Wrap
        }
    }
}
