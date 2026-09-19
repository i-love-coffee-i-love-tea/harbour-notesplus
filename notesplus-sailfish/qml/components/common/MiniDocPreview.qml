import QtQuick 2.6
import Sailfish.Silica 1.0
import "../editor"

Item {
    id: miniDocPreview
    property var previewBlocks: []
    property string previewBlocksJson: ""
    property var rawBlocks: []
    property string snippet: ""
    property bool highlighted: false
    property bool showBorder: true
    property real contentScale: (typeof app !== "undefined" && app && app.previewScale !== undefined) ? app.previewScale : 0.52
    property real previewHeight: width  // square by default

    implicitHeight: previewHeight

    property var parsedBlocks: {
        if (previewBlocks && previewBlocks.length > 0) {
            var list = []
            for (var k = 0; k < previewBlocks.length; k++) {
                var p = previewBlocks[k]
                if (typeof p === "string") {
                    try { list.push(JSON.parse(p)) } catch(e) {}
                } else if (typeof p === "object" && p !== null) {
                    list.push(p)
                }
            }
            return list
        }
        var list = []
        if (previewBlocksJson && previewBlocksJson.length > 0) {
            try {
                var arr = JSON.parse(previewBlocksJson)
                for (var i = 0; i < arr.length; i++) {
                    var itm = arr[i]
                    if (typeof itm === "string") {
                        try { list.push(JSON.parse(itm)) } catch(e) {}
                    } else if (typeof itm === "object" && itm !== null) {
                        list.push(itm)
                    }
                }
            } catch(e) {}
        }
        if (list.length === 0 && rawBlocks && rawBlocks.length > 0) {
            for (var j = 0; j < rawBlocks.length; j++) {
                var r = rawBlocks[j]
                if (typeof r === "string") {
                    try { list.push(JSON.parse(r)) } catch(e) {}
                } else if (typeof r === "object" && r !== null) {
                    list.push(r)
                }
            }
        }
        return list
    }

    Rectangle {
        id: cardRect
        anchors.fill: parent
        radius: showBorder ? Theme.paddingSmall : 0
        color: showBorder ? (highlighted ? Theme.rgba(Theme.highlightColor, 0.12) : Theme.rgba(Theme.primaryColor, 0.04)) : "transparent"
        border.color: showBorder ? (highlighted ? Theme.rgba(Theme.highlightColor, 0.5) : Theme.rgba(Theme.primaryColor, 0.15)) : "transparent"
        border.width: showBorder ? 1 : 0
        clip: true

        Item {
            id: scaledWrapper
            x: showBorder ? Theme.paddingSmall : 0
            y: showBorder ? Theme.paddingSmall : 0
            width: Math.max(10, (cardRect.width - (showBorder ? Theme.paddingSmall * 2 : 0)) / miniDocPreview.contentScale)
            height: Math.max(10, (cardRect.height - (showBorder ? Theme.paddingSmall * 2 : 0)) / miniDocPreview.contentScale)
            scale: miniDocPreview.contentScale
            transformOrigin: Item.TopLeft

            Column {
                id: contentColumn
                width: parent.width
                spacing: Theme.paddingSmall

                // Fallback snippet when parsed blocks are empty
                Label {
                    width: parent.width - Theme.horizontalPageMargin * 2
                    x: Theme.horizontalPageMargin
                    visible: parsedBlocks.length === 0 && snippet.length > 0
                    text: snippet
                    textFormat: Text.RichText
                    font.pixelSize: Theme.fontSizeSmall
                    color: Theme.secondaryColor
                    wrapMode: Text.Wrap
                    maximumLineCount: 6
                    truncationMode: TruncationMode.Elide
                }

                Label {
                    width: parent.width - Theme.horizontalPageMargin * 2
                    x: Theme.horizontalPageMargin
                    visible: parsedBlocks.length === 0 && snippet.length === 0
                    text: qsTr("Empty document")
                    font.italic: true
                    font.pixelSize: Theme.fontSizeSmall
                    color: Theme.secondaryColor
                }

                // Live miniaturized rendered blocks reusing BlockDelegate
                Repeater {
                    model: parsedBlocks.slice(0, 8)

                    delegate: BlockDelegate {
                        width: contentColumn.width
                        blockData: modelData
                        blockIndex: index
                        interactive: false
                        isTocCollapsed: {
                            var threshold = (typeof app !== "undefined" && app && app.tocCollapseThreshold !== undefined) ? app.tocCollapseThreshold : 5;
                            if (threshold >= 20) return false;
                            if (threshold === 0) return true;
                            var headings = (modelData && modelData.headings) ? modelData.headings : [];
                            return headings.length > threshold;
                        }
                    }
                }
            }
        }
    }
}
