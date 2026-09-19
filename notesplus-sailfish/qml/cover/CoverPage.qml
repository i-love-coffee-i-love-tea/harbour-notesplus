import QtQuick 2.6
import Sailfish.Silica 1.0

CoverBackground {
    id: cover

    Column {
        anchors.centerIn: parent
        spacing: Theme.paddingMedium

        Label {
            text: "Notes Plus"
            font.pixelSize: Theme.fontSizeLarge
            color: Theme.highlightColor
            anchors.horizontalCenter: parent.horizontalCenter
        }

        Label {
            text: bridge.current_page_name || qsTr("No page open")
            font.pixelSize: Theme.fontSizeSmall
            color: Theme.secondaryColor
            anchors.horizontalCenter: parent.horizontalCenter
            visible: bridge.current_page_name.length > 0
        }

        Label {
            text: qsTr("Web: ") + bridge.web_server_url
            font.pixelSize: Theme.fontSizeExtraSmall
            color: Theme.secondaryHighlightColor
            anchors.horizontalCenter: parent.horizontalCenter
            visible: bridge.web_server_running
        }
    }

    CoverActionList {
        id: coverActions

        CoverAction {
            iconSource: "image://theme/icon-cover-search"
            onTriggered: {
                app.openSearch()
            }
        }

        CoverAction {
            iconSource: "image://theme/icon-cover-new"
            onTriggered: {
                if (typeof app !== "undefined" && app && app.journalEnabled) {
                    app.openJournal()
                } else {
                    app.openSearch()
                }
            }
        }
    }
}
