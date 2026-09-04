import QtQuick 2.6
import Sailfish.Silica 1.0

Page {
    id: searchResultsPage
    allowedOrientations: Orientation.All

    SilicaListView {
        anchors.fill: parent
        model: bridge.search_results

        header: PageHeader {
            title: "Search Results"
        }

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
                    maximumLineCount: 2
                    textFormat: Text.RichText
                }
            }
        }

        ViewPlaceholder {
            enabled: bridge.search_results.length === 0
            text: "No results"
            hintText: "Try a different search term"
        }
    }
}
