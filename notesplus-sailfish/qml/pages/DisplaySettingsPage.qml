import QtQuick 2.6
import Sailfish.Silica 1.0
import "../components/settings"

Page {
    id: displaySettingsPage
    allowedOrientations: Orientation.All

    SilicaFlickable {
        anchors.fill: parent
        contentHeight: column.height + Theme.paddingLarge

        Column {
            id: column
            width: parent.width
            spacing: Theme.paddingMedium

            PageHeader {
                title: qsTr("Display")
            }

            DisplaySettingsTab {}
        }
    }
}
