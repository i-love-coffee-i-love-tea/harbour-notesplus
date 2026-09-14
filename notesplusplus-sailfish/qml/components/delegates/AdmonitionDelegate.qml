import QtQuick 2.6
import Sailfish.Silica 1.0
import "../"
import "../../js/BlockHtmlUtils.js" as BlockHtmlUtils

Item {
    id: admonitionDelegate
    property var blockData: ({})
    property int blockIndex: -1
    property string searchTerm: ""
    property int activeMatchIndexInBlock: -1
    signal xrefActivated(string target)
    signal checkboxToggled(int blockIndex, string itemPath)
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
        radius: 0
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
                font.family: app.resolvedFontFamily()
                font.pixelSize: app.scaledFontSize(Theme.fontSizeSmall)
                color: borderColor
            }

            Label {
                text: {
                    var html = (blockData && blockData.html) ? blockData.html : ""
                    // Strip outer wrapper (border, background, kind label) — QML provides these
                    html = html.replace(/<div[^>]*>/, "").replace(/<\/div>$/, "")
                    html = html.replace(/<b[^>]*>[^<]*:<\/b>\s*/, "")
                    html = html.replace(/__LINK_COLOR__/g, Theme.highlightColor)
                    if (searchTerm && searchTerm.length > 0) html = BlockHtmlUtils.highlightSearchTerms(html, searchTerm, activeMatchIndexInBlock)
                    return html
                }
                width: parent.width
                wrapMode: Text.WrapAtWordBoundaryOrAnywhere
                textFormat: Text.RichText
                font.family: app.resolvedFontFamily()
                font.pixelSize: app.scaledFontSize(Theme.fontSizeSmall)
                color: Theme.primaryColor
                linkColor: Theme.highlightColor
                onLinkActivated: function(link) {
                    BlockHtmlUtils.handleLink(link, function(target) { admonitionDelegate.xrefActivated(target) }, function(path) { admonitionDelegate.checkboxToggled(admonitionDelegate.blockIndex, path) })
                }
            }
        }
    }
}
