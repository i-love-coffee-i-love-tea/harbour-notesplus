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
                text: "AI Assistant"
                onClicked: {
                    pageStack.push(Qt.resolvedUrl("AssistantPage.qml"))
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
                title: "Notes++"
            }

            Item {
                width: parent.width
                height: bridge.web_server_running ? (webStatusRow.height + Theme.paddingSmall) : 0
                visible: bridge.web_server_running
                clip: true

                Behavior on height { NumberAnimation { duration: 150 } }

                BackgroundItem {
                    id: webStatusRow
                    anchors.centerIn: parent
                    width: parent.width - Theme.horizontalPageMargin * 2
                    height: Theme.itemSizeExtraSmall

                    Row {
                        anchors.centerIn: parent
                        spacing: Theme.paddingSmall

                        Rectangle {
                            width: Theme.paddingSmall
                            height: Theme.paddingSmall
                            radius: width / 2
                            color: "#4cd964"
                            anchors.verticalCenter: parent.verticalCenter
                        }

                        Label {
                            text: "Web Service: " + bridge.web_server_url
                            font.pixelSize: Theme.fontSizeExtraSmall
                            color: Theme.secondaryHighlightColor
                            anchors.verticalCenter: parent.verticalCenter
                        }
                    }

                    onClicked: {
                        bridge.open_in_browser("")
                    }
                }
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

            NoteCardGrid {
                id: searchGrid
                width: parent.width
                isPortraitOrientation: isPortrait
                model: searchField.text.length > 0 ? parsedSearchResults : []
                searchTerm: searchField.text
                visible: searchField.text.length > 0 && parsedSearchResults.length > 0
                onItemClicked: function(itemData, itemIndex) {
                    pageStack.push(Qt.resolvedUrl("PageView.qml"), {
                        pageName: itemData.name,
                        searchTerm: searchField.text
                    })
                    bridge.load_page(itemData.name)
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

            NoteCardGrid {
                id: recentGrid
                width: parent.width
                isPortraitOrientation: isPortrait
                model: searchField.text.length === 0 ? parsedRecentPages : []
                visible: searchField.text.length === 0
                onItemClicked: function(itemData, itemIndex) {
                    pageStack.push(Qt.resolvedUrl("PageView.qml"), {
                        pageName: itemData.name
                    })
                    bridge.load_page(itemData.name)
                }
            }
        }
    }

    RemorsePopup { id: remorsePopup }
}
