import QtQuick 2.6
import Sailfish.Silica 1.0
import "../"

InlineText {
    id: paragraph
    property var blockData: ({})

    spans: (blockData && blockData.spans) ? blockData.spans : []
    anchors.left: parent ? parent.left : undefined
    anchors.right: parent ? parent.right : undefined
    anchors.leftMargin: Theme.horizontalPageMargin
    anchors.rightMargin: Theme.horizontalPageMargin
    font.family: (typeof app !== "undefined" && app && app.docFontFamily && app.docFontFamily.length > 0) ? app.docFontFamily : Theme.fontFamily
    font.pixelSize: Math.round(Theme.fontSizeMedium * ((typeof app !== "undefined" && app && app.fontScale) ? app.fontScale : 1.0))
}
