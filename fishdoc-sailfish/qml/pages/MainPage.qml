import QtQuick 2.6
import Sailfish.Silica 1.0

Page {
    id: mainPage
    allowedOrientations: Orientation.All

    SilicaFlickable {
        anchors.fill: parent
        contentHeight: column.height

        PullDownMenu {
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
                    pageStack.push(Qt.resolvedUrl("PageView.qml"))
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

            // Search results
            Repeater {
                model: bridge.search_results.length > 0 && searchField.text.length > 0 ? bridge.search_results : 0

                delegate: ListItem {
                    contentHeight: Theme.itemSizeMedium
                    onClicked: {
                        pageStack.push(Qt.resolvedUrl("PageView.qml"))
                        bridge.load_page(modelData.name)
                    }

                    Column {
                        anchors {
                            left: parent.left
                            right: parent.right
                            leftMargin: Theme.horizontalPageMargin
                            rightMargin: Theme.horizontalPageMargin
                            verticalCenter: parent.verticalCenter
                        }

                        Label {
                            width: parent.width
                            text: modelData.name
                            color: highlighted ? Theme.highlightColor : Theme.primaryColor
                            font.pixelSize: Theme.fontSizeMedium
                        }

                        Label {
                            width: parent.width
                            text: modelData.snippet || ""
                            color: Theme.secondaryColor
                            font.pixelSize: Theme.fontSizeSmall
                            truncationMode: TruncationMode.Fade
                            maximumLineCount: 1
                            textFormat: Text.RichText
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
                        pageStack.push(Qt.resolvedUrl("PageView.qml"))
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

            Repeater {
                model: searchField.text.length === 0 ? bridge.recent_pages : 0

                delegate: ListItem {
                    contentHeight: Theme.itemSizeMedium
                    visible: searchField.text.length === 0
                    onClicked: {
                        pageStack.push(Qt.resolvedUrl("PageView.qml"))
                        bridge.load_page(modelData.name)
                    }

                    Label {
                        anchors {
                            left: parent.left
                            right: parent.right
                            leftMargin: Theme.horizontalPageMargin
                            rightMargin: Theme.horizontalPageMargin
                            verticalCenter: parent.verticalCenter
                        }
                        text: modelData.name
                        color: highlighted ? Theme.highlightColor : Theme.primaryColor
                        font.pixelSize: Theme.fontSizeMedium
                    }
                }
            }
        }
    }
}
