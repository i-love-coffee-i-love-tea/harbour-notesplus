import QtQuick 2.6
import Sailfish.Silica 1.0
import "../components"

Page {
    id: mainPage
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

    property var parsedRecentPages: {
        var list = []
        for (var i = 0; i < bridge.recent_pages.length; i++) {
            var str = bridge.recent_pages[i]
            if (typeof str === "string") {
                try { list.push(JSON.parse(str)) } catch(e) {}
            } else if (typeof str === "object" && str !== null) {
                list.push(str)
            }
        }
        return list
    }

    function activateSearch() {
        searchField.forceActiveFocus()
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

        PullDownMenu {
            MenuItem {
                text: "Settings"
                onClicked: {
                    pageStack.push(Qt.resolvedUrl("SettingsPage.qml"))
                }
            }
            MenuItem {
                text: "New Page"
                onClicked: {
                    var dialog = pageStack.push(Qt.resolvedUrl("NewPageDialog.qml"))
                    dialog.accepted.connect(function() {
                        if (dialog.pageName.length > 0) {
                            bridge.create_page(dialog.pageName)
                        }
                    })
                }
            }
            MenuItem {
                text: "Journal"
                onClicked: {
                    pageStack.push(Qt.resolvedUrl("PageView.qml"), {
                        pageName: "Journal"
                    })
                    bridge.load_page("Journal")
                }
            }
        }

        Column {
            id: column
            width: parent.width

            PageHeader {
                title: "FishDoc"
            }

            SearchField {
                id: searchField
                width: parent.width
                placeholderText: "Search pages..."
                onTextChanged: {
                    if (text.length > 0) {
                        bridge.do_search(text)
                    } else {
                        bridge.search("")
                    }
                }
            }

            // Search results section
            SectionHeader {
                text: "Search Results (" + parsedSearchResults.length + ")"
                visible: searchField.text.length > 0
            }

            Grid {
                id: searchGrid
                width: parent.width
                columns: isPortrait ? 2 : (width > 1200 ? 4 : 3)
                spacing: 0
                visible: searchField.text.length > 0 && parsedSearchResults.length > 0

                property real cellWidth: Math.floor(width / columns)

                Repeater {
                    model: searchField.text.length > 0 ? parsedSearchResults : 0

                    delegate: BackgroundItem {
                        id: searchGridItem
                        width: (index % searchGrid.columns === searchGrid.columns - 1) ? (searchGrid.width - searchGrid.cellWidth * (searchGrid.columns - 1)) : searchGrid.cellWidth
                        height: searchGrid.cellWidth
                        onClicked: {
                            pageStack.push(Qt.resolvedUrl("PageView.qml"), {
                                pageName: modelData.name,
                                searchTerm: searchField.text
                            })
                            bridge.load_page(modelData.name)
                        }

                        Rectangle {
                            id: searchCardBox
                            anchors.fill: parent
                            color: searchGridItem.highlighted ? Theme.rgba(Theme.highlightBackgroundColor, 0.22) : Theme.rgba(Theme.highlightBackgroundColor, 0.05)
                            border.color: searchGridItem.highlighted ? Theme.highlightColor : Theme.rgba(Theme.primaryColor, 0.12)
                            border.width: 1

                            MiniDocPreview {
                                anchors {
                                    left: parent.left
                                    right: parent.right
                                    top: parent.top
                                    bottom: searchCardFooter.top
                                    leftMargin: Theme.paddingMedium
                                    rightMargin: Theme.paddingMedium
                                    topMargin: Theme.paddingMedium
                                    bottomMargin: 0
                                }
                                showBorder: false
                                previewBlocksJson: modelData ? (modelData.preview_blocks_json || "") : ""
                                snippet: modelData ? (modelData.snippet || "") : ""
                                highlighted: searchGridItem.highlighted
                            }

                            Item {
                                id: searchCardFooter
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
                                    id: searchColorBar
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
                                    id: searchIndexLabel
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

            // Journal section
            SectionHeader {
                text: "Journal"
                visible: searchField.text.length === 0
            }

            Repeater {
                model: searchField.text.length === 0 ? bridge.recent_journal_lines : 0

                delegate: ListItem {
                    contentHeight: Theme.itemSizeSmall
                    visible: searchField.text.length === 0
                    onClicked: {
                        pageStack.push(Qt.resolvedUrl("PageView.qml"), {
                            pageName: "Journal"
                        })
                        bridge.load_page("Journal")
                    }

                    Label {
                        anchors {
                            left: parent.left
                            right: parent.right
                            leftMargin: Theme.horizontalPageMargin
                            rightMargin: Theme.horizontalPageMargin
                            verticalCenter: parent.verticalCenter
                        }
                        text: modelData
                        color: Theme.primaryColor
                        font.pixelSize: Theme.fontSizeSmall
                        truncationMode: TruncationMode.Fade
                        maximumLineCount: 1
                    }
                }
            }

            // Recent pages section
            SectionHeader {
                text: "Recent Pages"
                visible: searchField.text.length === 0
            }

            Grid {
                id: recentGrid
                width: parent.width
                columns: isPortrait ? 2 : (width > 1200 ? 4 : 3)
                spacing: 0
                visible: searchField.text.length === 0

                property real cellWidth: Math.floor(width / columns)

                Repeater {
                    model: searchField.text.length === 0 ? parsedRecentPages : 0

                    delegate: BackgroundItem {
                        id: recentGridItem
                        width: (index % recentGrid.columns === recentGrid.columns - 1) ? (recentGrid.width - recentGrid.cellWidth * (recentGrid.columns - 1)) : recentGrid.cellWidth
                        height: recentGrid.cellWidth
                        visible: searchField.text.length === 0
                        onClicked: {
                            pageStack.push(Qt.resolvedUrl("PageView.qml"), {
                                pageName: modelData.name
                            })
                            bridge.load_page(modelData.name)
                        }

                        Rectangle {
                            id: recentCardBox
                            anchors.fill: parent
                            color: recentGridItem.highlighted ? Theme.rgba(Theme.highlightBackgroundColor, 0.22) : Theme.rgba(Theme.highlightBackgroundColor, 0.05)
                            border.color: recentGridItem.highlighted ? Theme.highlightColor : Theme.rgba(Theme.primaryColor, 0.12)
                            border.width: 1

                            MiniDocPreview {
                                anchors {
                                    left: parent.left
                                    right: parent.right
                                    top: parent.top
                                    bottom: recentCardFooter.top
                                    leftMargin: Theme.paddingMedium
                                    rightMargin: Theme.paddingMedium
                                    topMargin: Theme.paddingMedium
                                    bottomMargin: 0
                                }
                                showBorder: false
                                previewBlocksJson: modelData ? (modelData.preview_blocks_json || "") : ""
                                snippet: modelData ? (modelData.snippet || "") : ""
                                highlighted: recentGridItem.highlighted
                            }

                            Item {
                                id: recentCardFooter
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
                                    id: recentColorBar
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
                                    id: recentIndexLabel
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
        }
    }
}
