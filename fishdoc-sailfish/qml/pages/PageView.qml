import QtQuick 2.6
import Sailfish.Silica 1.0
import "../components"

Page {
    id: pageView
    allowedOrientations: Orientation.All

    property string pageName: bridge.current_page_name

    SilicaListView {
        id: listView
        anchors.fill: parent
        model: bridge.current_blocks

        PullDownMenu {
            MenuItem {
                text: "Export to PDF"
                visible: false // Phase 5
            }
            MenuItem {
                text: "Delete Page"
                visible: !bridge.is_journal_page
                onClicked: {
                    remorse.execute("Deleting", function() {
                        bridge.delete_page(pageName)
                        pageStack.pop()
                    })
                }
            }
        }

        header: PageHeader {
            title: pageName
        }

        delegate: BlockDelegate {
            width: listView.width
            blockData: modelData ? JSON.parse(modelData) : ({})
            onTapEdit: {
                // TODO: re-enable editing once save architecture is fixed
            }
            onLinkActivated: function(target) {
                bridge.navigate_to_page(target)
            }
        }

        RemorseItem { id: remorse }
    }
}
