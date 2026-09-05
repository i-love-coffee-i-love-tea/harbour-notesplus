import QtQuick 2.6
import Sailfish.Silica 1.0
import "../components"

Page {
    id: searchResultsPage
    allowedOrientations: Orientation.All

    property var parsedSearchResults: {
        var list = []
        for (var i = 0; i < bridge.search_results.length; i++) {
            var str = bridge.search_results[i]
            if (typeof str === "string") {
                try { list.push(JSON.parse(str)) } catch(e) {}
            } else if (typeof str === "object" && str !== null) {
                list.push(str)
            }
        }
        return list
    }

    function getNoteColor(name) {
        var palette = [
            "#e67e22", // orange
            "#3498db", // blue
            "#2ecc71", // green
            "#9b59b6", // purple
            "#f1c40f", // yellow
            "#e74c3c", // red
            "#1abc9c", // teal
            "#e84393", // pink
            "#00cec9", // cyan
            "#6c5ce7", // indigo
            "#fdcb6e", // amber
            "#00b894"  // emerald
        ];
        var hash = 0;
        if (name) {
            for (var i = 0; i < name.length; i++) {
                hash = (hash * 31 + name.charCodeAt(i)) & 0x7FFFFFFF;
            }
        }
        return palette[hash % palette.length];
    }

    SilicaFlickable {
        anchors.fill: parent
        contentHeight: column.height + Theme.paddingLarge

        Column {
            id: column
            width: parent.width

            PageHeader {
                title: "Search Results"
            }

            Grid {
                id: resultsGrid
                width: parent.width
                columns: isPortrait ? 2 : (width > 1200 ? 4 : 3)
                spacing: 0
                visible: parsedSearchResults.length > 0

                property real cellWidth: Math.floor(width / columns)

                Repeater {
                    model: parsedSearchResults

                    delegate: BackgroundItem {
                        id: resultGridItem
                        width: (index % resultsGrid.columns === resultsGrid.columns - 1) ? (resultsGrid.width - resultsGrid.cellWidth * (resultsGrid.columns - 1)) : resultsGrid.cellWidth
                        height: resultsGrid.cellWidth
                        onClicked: {
                            pageStack.push(Qt.resolvedUrl("PageView.qml"), {
                                pageName: modelData.name,
                                searchTerm: modelData.query || ""
                            })
                            bridge.load_page(modelData.name)
                        }

                        Rectangle {
                            id: resultCardBox
                            anchors.fill: parent
                            color: resultGridItem.highlighted ? Theme.rgba(Theme.highlightBackgroundColor, 0.22) : Theme.rgba(Theme.highlightBackgroundColor, 0.05)
                            border.color: resultGridItem.highlighted ? Theme.highlightColor : Theme.rgba(Theme.primaryColor, 0.12)
                            border.width: 1

                            MiniDocPreview {
                                anchors {
                                    left: parent.left
                                    right: parent.right
                                    top: parent.top
                                    bottom: resultCardFooter.top
                                    leftMargin: Theme.paddingMedium
                                    rightMargin: Theme.paddingMedium
                                    topMargin: Theme.paddingMedium
                                    bottomMargin: 0
                                }
                                showBorder: false
                                previewBlocksJson: modelData ? (modelData.preview_blocks_json || "") : ""
                                snippet: modelData ? (modelData.snippet || "") : ""
                                highlighted: resultGridItem.highlighted
                            }

                            Item {
                                id: resultCardFooter
                                anchors {
                                    left: parent.left
                                    right: parent.right
                                    bottom: parent.bottom
                                    leftMargin: Theme.paddingMedium
                                    rightMargin: Theme.paddingMedium
                                    bottomMargin: Theme.paddingSmall
                                }
                                height: Theme.paddingLarge

                                Rectangle {
                                    id: resultColorBar
                                    anchors {
                                        left: parent.left
                                        verticalCenter: parent.verticalCenter
                                    }
                                    width: Math.round(Theme.itemSizeExtraSmall * 0.6)
                                    height: 4
                                    radius: 2
                                    color: getNoteColor(modelData ? modelData.name : "")
                                }

                                Label {
                                    id: resultIndexLabel
                                    anchors {
                                        right: parent.right
                                        verticalCenter: parent.verticalCenter
                                    }
                                    text: (index + 1).toString()
                                    font.pixelSize: Theme.fontSizeExtraSmall
                                    color: Theme.secondaryColor
                                }
                            }
                        }
                    }
                }
            }

            ViewPlaceholder {
                enabled: parsedSearchResults.length === 0
                text: "No results"
                hintText: "Try a different search term"
            }
        }
    }
}
