import QtQuick 2.6
import Sailfish.Silica 1.0
import "../"

InlineText {
    id: heading
    property var blockData: ({})
    property int level: (blockData && blockData.level) ? blockData.level : 1

    spans: (blockData && blockData.spans) ? blockData.spans : []
    color: Theme.highlightColor
    font.family: (typeof app !== "undefined" && app && app.docFontFamily && app.docFontFamily.length > 0) ? app.docFontFamily : Theme.fontFamily
    font.pixelSize: {
        var scale = (typeof app !== "undefined" && app && app.fontScale) ? app.fontScale : 1.0
        switch (level) {
            case 1: return Math.round(Theme.fontSizeExtraLarge * scale)
            case 2: return Math.round(Theme.fontSizeLarge * scale)
            case 3: return Math.round(Theme.fontSizeMedium * scale)
            default: return Math.round(Theme.fontSizeSmall * scale)
        }
    }
    font.bold: level <= 2
    wrapMode: Text.WrapAtWordBoundaryOrAnywhere
    anchors.left: parent ? parent.left : undefined
    anchors.right: parent ? parent.right : undefined
    anchors.leftMargin: Theme.horizontalPageMargin
    anchors.rightMargin: Theme.horizontalPageMargin
}
