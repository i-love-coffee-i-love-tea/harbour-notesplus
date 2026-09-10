import QtQuick 2.6
import Sailfish.Silica 1.0

Rectangle {
    id: infoCard

    default property alias content: contentColumn.children

    width: parent.width - Theme.horizontalPageMargin * 2
    anchors.horizontalCenter: parent.horizontalCenter
    height: contentColumn.height + Theme.paddingMedium * 2
    color: Theme.rgba(Theme.highlightBackgroundColor, 0.1)
    border.color: Theme.rgba(Theme.highlightColor, 0.3)
    border.width: 1
    radius: Theme.paddingSmall

    Column {
        id: contentColumn
        anchors {
            left: parent.left
            right: parent.right
            top: parent.top
            margins: Theme.paddingMedium
        }
        spacing: Theme.paddingSmall
    }
}
