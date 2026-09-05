import QtQuick 2.6
import Sailfish.Silica 1.0
import "../"
import "../../js/BlockHtmlUtils.js" as BlockHtmlUtils

Item {
    id: admonitionDelegate
    property var blockData: ({})
    signal xrefActivated(string target)
    property string kind: (blockData && blockData.kind) ? blockData.kind : "NOTE"
    property color borderColor: {
        switch (kind) {
            case "WARNING": return "#d9534f"
            case "TIP": return "#f0ad4e"
            default: return Theme.highlightColor
        }
    }
    property color bgColor: {
        switch (kind) {
            case "WARNING": return Qt.rgba(0.85, 0.33, 0.31, 0.12)
            case "TIP": return Qt.rgba(0.94, 0.68, 0.31, 0.12)
            default: return Qt.rgba(Theme.highlightBackgroundColor.r, Theme.highlightBackgroundColor.g, Theme.highlightBackgroundColor.b, 0.12)
        }
    }
    property string iconSource: {
        switch (kind) {
            case "WARNING": return "image://theme/icon-s-warning?" + borderColor
            case "TIP": return "image://theme/icon-s-high-importance?" + borderColor
            default: return "image://theme/icon-m-about?" + borderColor
        }
    }

    anchors.left: parent ? parent.left : undefined
    anchors.right: parent ? parent.right : undefined
    anchors.leftMargin: Theme.horizontalPageMargin
    anchors.rightMargin: Theme.horizontalPageMargin
    height: admonitionRect.height

    Rectangle {
        id: admonitionRect
        anchors.left: parent.left
        anchors.right: parent.right
        color: bgColor
        border.color: borderColor
        border.width: 2
        radius: 4
        height: Math.max(iconImage.height + Theme.paddingMedium * 2, admonitionColumn.height + Theme.paddingMedium * 2)

        Image {
            id: iconImage
            anchors {
                left: parent.left
                top: parent.top
                margins: Theme.paddingMedium
            }
            source: iconSource
            width: Theme.iconSizeSmall
            height: Theme.iconSizeSmall
            sourceSize.width: Theme.iconSizeSmall
            sourceSize.height: Theme.iconSizeSmall
        }

        Column {
            id: admonitionColumn
            anchors {
                left: iconImage.right
                right: parent.right
                top: parent.top
                margins: Theme.paddingMedium
                leftMargin: Theme.paddingMedium
            }
            spacing: Theme.paddingSmall

            Label {
                text: (blockData && blockData.title && blockData.title.length > 0) ? (kind + ": " + blockData.title) : kind
                font.bold: true
                font.family: (typeof app !== "undefined" && app && app.docFontFamily && app.docFontFamily.length > 0) ? app.docFontFamily : Theme.fontFamily
                font.pixelSize: Math.round(Theme.fontSizeSmall * ((typeof app !== "undefined" && app && app.fontScale) ? app.fontScale : 1.0))
                color: borderColor
            }

            Label {
                text: BlockHtmlUtils.blocksToHtml((blockData && blockData.blocks) ? blockData.blocks : [], 0, "", { highlightColor: Theme.highlightColor, primaryColor: Theme.primaryColor, highlightBackgroundColor: Theme.highlightBackgroundColor }, (typeof bridge !== "undefined" && bridge) ? bridge.notes_dir : "", (typeof app !== "undefined" && app) ? app.allowExternalImages : true)
                width: parent.width
                wrapMode: Text.WrapAtWordBoundaryOrAnywhere
                textFormat: Text.RichText
                font.family: (typeof app !== "undefined" && app && app.docFontFamily && app.docFontFamily.length > 0) ? app.docFontFamily : Theme.fontFamily
                font.pixelSize: Math.round(Theme.fontSizeSmall * ((typeof app !== "undefined" && app && app.fontScale) ? app.fontScale : 1.0))
                color: Theme.primaryColor
                linkColor: Theme.highlightColor
                onLinkActivated: function(link) {
                    if (link.indexOf("xref:") === 0) {
                        var target = link.substring(5)
                        if (target.indexOf(".adoc") === target.length - 5) {
                            target = target.substring(0, target.length - 5)
                        }
                        admonitionDelegate.xrefActivated(target)
                    } else if (link.indexOf("http") === 0) {
                        Qt.openUrlExternally(link)
                    }
                }
            }
        }
    }
}
