import QtQuick 2.6
import Sailfish.Silica 1.0
import "../"
import "../../common"
import "../../../js/BlockHtmlUtils.js" as BlockHtmlUtils

Item {
    id: listDelegateItem
    property var blockData: ({})
    property var localBlockData: blockData
    property int blockIndex: -1
    property int renderCounter: 0
    property string searchTerm: ""
    property int activeMatchIndexInBlock: -1

    signal xrefActivated(string target)
    signal checkboxToggled(int blockIndex, string itemPath)
    signal toggleLocalCheckbox(string path)

    height: listItem.height + Theme.paddingSmall
    anchors.left: parent ? parent.left : undefined
    anchors.right: parent ? parent.right : undefined

    InlineText {
        id: listItem
        visible: true
        text: {
            if (blockData && blockData.html) {
                var html = blockData.html.replace(/__LINK_COLOR__/g, Theme.highlightColor)
                if (searchTerm && searchTerm.length > 0) {
                    html = BlockHtmlUtils.highlightSearchTerms(html, searchTerm, activeMatchIndexInBlock)
                }
                return html
            }
            return ""
        }
        font.family: app.resolvedFontFamily()
        font.pixelSize: app.scaledFontSize(Theme.fontSizeSmall)
        anchors.left: parent ? parent.left : undefined
        anchors.right: parent ? parent.right : undefined
        anchors.leftMargin: Theme.horizontalPageMargin + ((localBlockData && localBlockData.level) ? (localBlockData.level * Theme.paddingLarge) : 0)
        anchors.rightMargin: Theme.horizontalPageMargin
        blockIndex: listDelegateItem.blockIndex
        onXrefActivated: function(target) { listDelegateItem.xrefActivated(target) }
        onCheckboxToggled: function(idx, itemPath) {
            listDelegateItem.toggleLocalCheckbox(itemPath)
            listDelegateItem.checkboxToggled(idx, itemPath)
        }
    }
}
