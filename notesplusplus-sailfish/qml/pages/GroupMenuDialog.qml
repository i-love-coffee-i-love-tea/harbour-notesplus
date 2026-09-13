import QtQuick 2.6
import Sailfish.Silica 1.0

Page {
    id: groupMenuPage
    allowedOrientations: Orientation.All

    property string groupPath: ""
    property string displayName: ""

    signal actionSelected(string action)

    SilicaFlickable {
        anchors.fill: parent
        contentHeight: column.height + Theme.paddingLarge

        PullDownMenu {
            MenuItem {
                text: qsTr("Cancel")
                onClicked: pageStack.pop()
            }
        }

        Column {
            id: column
            width: parent.width
            spacing: Theme.paddingMedium

            PageHeader {
                title: displayName || groupPath
            }

            BackgroundItem {
                width: parent.width
                height: Theme.itemSizeMedium
                onClicked: {
                    groupMenuPage.actionSelected("new_page")
                    pageStack.pop()
                }
                Row {
                    anchors.fill: parent
                    anchors.leftMargin: Theme.horizontalPageMargin
                    anchors.rightMargin: Theme.horizontalPageMargin
                    spacing: Theme.paddingMedium
                    Icon {
                        anchors.verticalCenter: parent.verticalCenter
                        source: "image://theme/icon-m-note"
                        color: parent.parent.highlighted ? Theme.highlightColor : Theme.primaryColor
                    }
                    Label {
                        anchors.verticalCenter: parent.verticalCenter
                        text: qsTr("New Note in Group")
                        color: parent.parent.highlighted ? Theme.highlightColor : Theme.primaryColor
                    }
                }
            }

            BackgroundItem {
                width: parent.width
                height: Theme.itemSizeMedium
                onClicked: {
                    groupMenuPage.actionSelected("new_subgroup")
                    pageStack.pop()
                }
                Row {
                    anchors.fill: parent
                    anchors.leftMargin: Theme.horizontalPageMargin
                    anchors.rightMargin: Theme.horizontalPageMargin
                    spacing: Theme.paddingMedium
                    Icon {
                        anchors.verticalCenter: parent.verticalCenter
                        source: "image://theme/icon-m-add"
                        color: parent.parent.highlighted ? Theme.highlightColor : Theme.primaryColor
                    }
                    Label {
                        anchors.verticalCenter: parent.verticalCenter
                        text: qsTr("New Subgroup")
                        color: parent.parent.highlighted ? Theme.highlightColor : Theme.primaryColor
                    }
                }
            }

            BackgroundItem {
                width: parent.width
                height: Theme.itemSizeMedium
                onClicked: {
                    groupMenuPage.actionSelected("rename")
                    pageStack.pop()
                }
                Row {
                    anchors.fill: parent
                    anchors.leftMargin: Theme.horizontalPageMargin
                    anchors.rightMargin: Theme.horizontalPageMargin
                    spacing: Theme.paddingMedium
                    Icon {
                        anchors.verticalCenter: parent.verticalCenter
                        source: "image://theme/icon-m-edit"
                        color: parent.parent.highlighted ? Theme.highlightColor : Theme.primaryColor
                    }
                    Label {
                        anchors.verticalCenter: parent.verticalCenter
                        text: qsTr("Rename Group")
                        color: parent.parent.highlighted ? Theme.highlightColor : Theme.primaryColor
                    }
                }
            }

            BackgroundItem {
                width: parent.width
                height: Theme.itemSizeMedium
                onClicked: {
                    groupMenuPage.actionSelected("delete")
                    pageStack.pop()
                }
                Row {
                    anchors.fill: parent
                    anchors.leftMargin: Theme.horizontalPageMargin
                    anchors.rightMargin: Theme.horizontalPageMargin
                    spacing: Theme.paddingMedium
                    Icon {
                        anchors.verticalCenter: parent.verticalCenter
                        source: "image://theme/icon-m-delete"
                        color: Theme.errorColor
                    }
                    Label {
                        anchors.verticalCenter: parent.verticalCenter
                        text: qsTr("Delete Group")
                        color: Theme.errorColor
                    }
                }
            }
        }
    }
}
