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

    SilicaFlickable {
        anchors.fill: parent
        contentHeight: column.height + Theme.paddingLarge

        Column {
            id: column
            width: parent.width

            PageHeader {
                title: "Search Results"
            }

            NoteCardGrid {
                id: resultsGrid
                width: parent.width
                isPortraitOrientation: isPortrait
                model: parsedSearchResults
                visible: parsedSearchResults.length > 0
                onItemClicked: function(itemData, itemIndex) {
                    pageStack.push(Qt.resolvedUrl("PageView.qml"), {
                        pageName: itemData.name,
                        searchTerm: itemData.query || ""
                    })
                    bridge.load_page(itemData.name)
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
