import QtQuick 2.6
import Sailfish.Silica 1.0
import "../"
import "../../js/BlockHtmlUtils.js" as BlockHtmlUtils

Item {
    id: listDelegateItem
    property var blockData: ({})
    property var localBlockData: blockData
    property int blockIndex: -1
    property int renderCounter: 0
    property string searchTerm: ""

    property string itemType: (blockData && blockData.type) ? blockData.type : "unordered_list_item"
    property bool isDescriptionList: itemType === "description_list_item"
    property bool isCalloutList: itemType === "callout_list_item"
    property bool isStandardList: !isDescriptionList && !isCalloutList

    signal xrefActivated(string target)
    signal checkboxToggled(int blockIndex, string itemPath)
    signal toggleLocalCheckbox(string path)

    height: isDescriptionList ? dlContainer.height : (isCalloutList ? calloutContainer.height : standardListItem.height)
    anchors.left: parent ? parent.left : undefined
    anchors.right: parent ? parent.right : undefined

    InlineText {
        id: standardListItem
        visible: listDelegateItem.isStandardList
        text: {
            // Use Rust pre-rendered HTML if available
            if (blockData && blockData.html && listDelegateItem.isStandardList) {
                var html = blockData.html
                if (listDelegateItem.searchTerm && listDelegateItem.searchTerm.length > 0) {
                    html = BlockHtmlUtils.highlightSearchTerms(html, listDelegateItem.searchTerm)
                }
                return html
            }
            var html = (listDelegateItem.isStandardList && listDelegateItem.renderCounter >= 0) ? BlockHtmlUtils.blocksToHtml([localBlockData], (localBlockData && localBlockData.level) ? localBlockData.level : 0, "", { highlightColor: Theme.highlightColor, primaryColor: Theme.primaryColor, highlightBackgroundColor: Theme.highlightBackgroundColor }, (typeof bridge !== "undefined" && bridge) ? bridge.notes_dir : "", (typeof app !== "undefined" && app) ? app.allowExternalImages : true) : ""
            if (listDelegateItem.searchTerm && listDelegateItem.searchTerm.length > 0 && html.length > 0) {
                html = BlockHtmlUtils.highlightSearchTerms(html, listDelegateItem.searchTerm)
            }
            return html
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

    Item {
        id: dlContainer
        visible: listDelegateItem.isDescriptionList
        anchors.left: parent ? parent.left : undefined
        anchors.right: parent ? parent.right : undefined
        anchors.leftMargin: Theme.horizontalPageMargin
        anchors.rightMargin: Theme.horizontalPageMargin
        height: dlCol.height

        Column {
            id: dlCol
            anchors.left: parent.left
            anchors.right: parent.right
            spacing: Theme.paddingSmall / 2

            Label {
                width: parent.width
                textFormat: Text.RichText
                text: listDelegateItem.isDescriptionList ? (blockData.term_spans ? ("<b>" + BlockHtmlUtils.spansToHtml(blockData.term_spans, { highlightColor: Theme.highlightColor, primaryColor: Theme.primaryColor, highlightBackgroundColor: Theme.highlightBackgroundColor }) + "</b>") : ("<b>" + BlockHtmlUtils.escapeHtml(blockData.term || "") + "</b>")) : ""
                color: Theme.primaryColor
                font.family: app.resolvedFontFamily()
                font.pixelSize: app.scaledFontSize(Theme.fontSizeMedium)
                wrapMode: Text.Wrap
            }

            Label {
                width: parent.width - Theme.paddingMedium
                x: Theme.paddingMedium
                wrapMode: Text.WrapAtWordBoundaryOrAnywhere
                textFormat: Text.RichText
                text: listDelegateItem.isDescriptionList ? BlockHtmlUtils.blocksToHtml(blockData.blocks || [], 0, "", { highlightColor: Theme.highlightColor, primaryColor: Theme.primaryColor, highlightBackgroundColor: Theme.highlightBackgroundColor }) : ""
                font.family: app.resolvedFontFamily()
                font.pixelSize: app.scaledFontSize(Theme.fontSizeSmall)
                color: Theme.primaryColor
                linkColor: Theme.highlightColor
                onLinkActivated: function(link) {
                    BlockHtmlUtils.handleLink(link, function(target) { listDelegateItem.xrefActivated(target) }, function(path) { listDelegateItem.checkboxToggled(listDelegateItem.blockIndex, path) })
                }
            }
        }
    }

    Item {
        id: calloutContainer
        visible: listDelegateItem.isCalloutList
        anchors.left: parent ? parent.left : undefined
        anchors.right: parent ? parent.right : undefined
        anchors.leftMargin: Theme.horizontalPageMargin
        anchors.rightMargin: Theme.horizontalPageMargin
        height: Math.max(badgeLabel.height, calloutText.height)

        Label {
            id: badgeLabel
            text: "<" + (blockData.number || 1) + ">"
            color: Theme.highlightColor
            font.bold: true
            font.pixelSize: app.scaledFontSize(Theme.fontSizeSmall)
            anchors.left: parent.left
            anchors.top: parent.top
        }

        Label {
            id: calloutText
            anchors.left: badgeLabel.right
            anchors.leftMargin: Theme.paddingMedium
            anchors.right: parent.right
            wrapMode: Text.WrapAtWordBoundaryOrAnywhere
            textFormat: Text.RichText
            font.family: app.resolvedFontFamily()
            font.pixelSize: app.scaledFontSize(Theme.fontSizeSmall)
            color: Theme.primaryColor
            linkColor: Theme.highlightColor
            text: listDelegateItem.isCalloutList ? BlockHtmlUtils.blocksToHtml(blockData.blocks || [], 0, "", { highlightColor: Theme.highlightColor, primaryColor: Theme.primaryColor, highlightBackgroundColor: Theme.highlightBackgroundColor }) : ""
            onLinkActivated: function(link) {
                BlockHtmlUtils.handleLink(link, function(target) { listDelegateItem.xrefActivated(target) }, function(path) { listDelegateItem.checkboxToggled(listDelegateItem.blockIndex, path) })
            }
        }
    }
}
