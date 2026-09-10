import QtQuick 2.6
import Sailfish.Silica 1.0
import "../"

InlineText {
    id: heading
    property var blockData: ({})
    property int level: (blockData && blockData.level) ? blockData.level : 1

    spans: (blockData && blockData.spans) ? blockData.spans : []
    color: Theme.highlightColor
    font.family: app.resolvedFontFamily()
    font.pixelSize: {
        switch (level) {
            case 1: return app.scaledFontSize(Theme.fontSizeExtraLarge)
            case 2: return app.scaledFontSize(Theme.fontSizeLarge)
            case 3: return app.scaledFontSize(Theme.fontSizeMedium)
            default: return app.scaledFontSize(Theme.fontSizeSmall)
        }
    }
    font.bold: level <= 2
    wrapMode: Text.WrapAtWordBoundaryOrAnywhere
    anchors.left: parent ? parent.left : undefined
    anchors.right: parent ? parent.right : undefined
    anchors.leftMargin: Theme.horizontalPageMargin
    anchors.rightMargin: Theme.horizontalPageMargin
}
