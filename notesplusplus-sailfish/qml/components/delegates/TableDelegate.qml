import QtQuick 2.6
import Sailfish.Silica 1.0
import "../"
import "../../js/BlockHtmlUtils.js" as BlockHtmlUtils

Column {
    id: tableDelegate
    property var blockData: ({})
    signal xrefActivated(string target)

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

    Column {
        id: tableGrid
        property int columnCount: {
            var maxCols = 1
            if (blockData && blockData.col_widths && blockData.col_widths.length > 0) {
                maxCols = blockData.col_widths.length
            }
            if (blockData && blockData.rows) {
                for (var r = 0; r < blockData.rows.length; r++) {
                    if (blockData.rows[r] && blockData.rows[r].length > maxCols) {
                        maxCols = blockData.rows[r].length
                    }
                }
            }
            return maxCols
        }
        property int rowCount: (blockData && blockData.rows) ? blockData.rows.length : 0
        property string frameStyle: (blockData && blockData.frame) ? blockData.frame.toLowerCase() : "all"
        property string gridStyle: (blockData && blockData.grid) ? blockData.grid.toLowerCase() : "all"

        property bool hasTopBorder: frameStyle === "all" || frameStyle === "topbot" || frameStyle === "ends" || frameStyle === "rows"
        property bool hasBottomBorder: frameStyle === "all" || frameStyle === "topbot" || frameStyle === "ends" || frameStyle === "rows"
        property bool hasLeftBorder: frameStyle === "all" || frameStyle === "sides" || frameStyle === "cols"
        property bool hasRightBorder: frameStyle === "all" || frameStyle === "sides" || frameStyle === "cols"
        property bool hasRowDividers: gridStyle === "all" || gridStyle === "rows"
        property bool hasColDividers: gridStyle === "all" || gridStyle === "cols"

        property color borderColor: Theme.rgba(Theme.primaryColor, 0.3)

        width: parent.width
        spacing: 0

        Repeater {
            model: (blockData && blockData.rows) ? blockData.rows : []
            delegate: Row {
                id: rowItem
                property int rowIndex: index
                property var rowCells: modelData || []
                property var cellHeights: ({})
                property real maxCellHeight: Math.round(Theme.itemSizeExtraSmall * 0.7)

                function updateCellHeight(colIdx, h) {
                    var copy = cellHeights
                    copy[colIdx] = h
                    var maxH = Math.round(Theme.itemSizeExtraSmall * 0.7)
                    for (var k in copy) {
                        if (copy[k] > maxH) maxH = copy[k]
                    }
                    maxCellHeight = maxH
                }

                width: parent.width
                height: maxCellHeight

                Repeater {
                    id: cellRepeater
                    model: rowCells
                    delegate: Item {
                        id: cellItem
                        property int colIndex: index
                        property real cellColSpan: (modelData && modelData.colspan) ? modelData.colspan : 1.0
                        width: {
                            var totalW = rowItem.width
                            var widths = (blockData && blockData.col_widths && blockData.col_widths.length > 0) ? blockData.col_widths : []
                            if (widths.length === tableGrid.columnCount) {
                                var sum = 0
                                for (var i = 0; i < widths.length; i++) sum += widths[i]
                                var cellRatioSum = 0
                                for (var c = 0; c < cellColSpan && (colIndex + c) < widths.length; c++) {
                                    cellRatioSum += widths[colIndex + c]
                                }
                                return sum > 0 ? (totalW * cellRatioSum / sum) : (totalW / tableGrid.columnCount * cellColSpan)
                            }
                            return (totalW / tableGrid.columnCount) * cellColSpan
                        }
                        property real naturalHeight: Math.max(cellText.height + 16, Math.round(Theme.itemSizeExtraSmall * 0.7))
                        onNaturalHeightChanged: rowItem.updateCellHeight(colIndex, naturalHeight)
                        Component.onCompleted: rowItem.updateCellHeight(colIndex, naturalHeight)

                        height: rowItem.maxCellHeight

                        Rectangle {
                            anchors.fill: parent
                            color: (rowIndex === 0 && modelData && modelData.header) ? Theme.rgba(Theme.highlightBackgroundColor, 0.15) : ((rowIndex % 2 === 1) ? Theme.rgba(Theme.primaryColor, 0.03) : "transparent")
                        }

                        Rectangle {
                            anchors.top: parent.top
                            anchors.left: parent.left
                            anchors.right: parent.right
                            height: 1
                            color: tableGrid.borderColor
                            visible: Boolean((rowIndex === 0 && tableGrid.hasTopBorder) || (rowIndex > 0 && tableGrid.hasRowDividers))
                        }

                        Rectangle {
                            anchors.bottom: parent.bottom
                            anchors.left: parent.left
                            anchors.right: parent.right
                            height: 1
                            color: tableGrid.borderColor
                            visible: Boolean(rowIndex === tableGrid.rowCount - 1 && tableGrid.hasBottomBorder)
                        }

                        Rectangle {
                            anchors.left: parent.left
                            anchors.top: parent.top
                            anchors.bottom: parent.bottom
                            width: 1
                            color: tableGrid.borderColor
                            visible: Boolean((colIndex === 0 && tableGrid.hasLeftBorder) || (colIndex > 0 && tableGrid.hasColDividers))
                        }

                        Rectangle {
                            anchors.right: parent.right
                            anchors.top: parent.top
                            anchors.bottom: parent.bottom
                            width: 1
                            color: tableGrid.borderColor
                            visible: Boolean(colIndex === tableGrid.columnCount - 1 && tableGrid.hasRightBorder)
                        }

                        Label {
                            id: cellText
                            anchors {
                                left: parent.left
                                right: parent.right
                                top: (modelData && modelData.valign === "bottom") ? undefined : ((modelData && modelData.valign === "middle") ? undefined : parent.top)
                                bottom: (modelData && modelData.valign === "bottom") ? parent.bottom : undefined
                                verticalCenter: (modelData && modelData.valign === "middle") ? parent.verticalCenter : undefined
                                leftMargin: 8
                                rightMargin: 8
                                topMargin: 8
                                bottomMargin: (modelData && modelData.valign === "bottom") ? 8 : 0
                            }
                            horizontalAlignment: {
                                if (modelData && modelData.align === "center") return Text.AlignHCenter
                                if (modelData && modelData.align === "right") return Text.AlignRight
                                return Text.AlignLeft
                            }
                            wrapMode: Text.WrapAtWordBoundaryOrAnywhere
                            font.family: (typeof app !== "undefined" && app && app.docFontFamily && app.docFontFamily.length > 0) ? app.docFontFamily : Theme.fontFamily
                            font.pixelSize: Math.round(Theme.fontSizeSmall * ((typeof app !== "undefined" && app && app.fontScale) ? app.fontScale : 1.0))
                            font.bold: Boolean(rowIndex === 0 || (modelData && modelData.header))
                            color: Theme.primaryColor
                            linkColor: Theme.highlightColor
                            textFormat: Text.RichText
                            text: {
                                var blocks = Array.isArray(modelData) ? modelData : (modelData && modelData.blocks ? modelData.blocks : [])
                                if (typeof blocks === "string") return blocks
                                return BlockHtmlUtils.blocksToHtml(blocks, 0, "", { highlightColor: Theme.highlightColor, primaryColor: Theme.primaryColor, highlightBackgroundColor: Theme.highlightBackgroundColor }, (typeof bridge !== "undefined" && bridge) ? bridge.notes_dir : "", (typeof app !== "undefined" && app) ? app.allowExternalImages : true)
                            }
                            onLinkActivated: function(link) {
                                if (link.indexOf("xref:") === 0) {
                                    var target = link.substring(5)
                                    if (target.indexOf(".adoc") === target.length - 5) {
                                        target = target.substring(0, target.length - 5)
                                    }
                                    tableDelegate.xrefActivated(target)
                                } else if (link.indexOf("http") === 0) {
                                    Qt.openUrlExternally(link)
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
