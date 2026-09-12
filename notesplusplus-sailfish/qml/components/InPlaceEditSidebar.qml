import QtQuick 2.6
import Sailfish.Silica 1.0

Item {
    id: inPlaceSidebar
    width: Theme.itemSizeSmall
    height: sidebarCol.height + Theme.paddingSmall * 2
    z: 30

    signal accepted()
    signal canceled()
    signal prefixRequested(string prefix, bool multiLine)
    signal linkRequested()
    signal pasteRequested()

    Rectangle {
        anchors.fill: parent
        radius: Theme.paddingMedium
        color: Theme.rgba(Theme.overlayBackgroundColor, 0.90)
        border.color: Theme.rgba(Theme.highlightColor, 0.3)
        border.width: 1
    }

    Column {
        id: sidebarCol
        anchors.centerIn: parent
        width: parent.width
        spacing: 2

        // --- Action Group (Check / Cancel) ---
        IconButton {
            id: acceptBtn
            icon.source: "image://theme/icon-m-accept"
            anchors.horizontalCenter: parent.horizontalCenter
            height: Theme.itemSizeExtraSmall
            width: Theme.itemSizeSmall
            onClicked: {
                inPlaceSidebar.accepted()
            }
        }

        IconButton {
            id: cancelBtn
            icon.source: "image://theme/icon-m-clear"
            anchors.horizontalCenter: parent.horizontalCenter
            height: Theme.itemSizeExtraSmall
            width: Theme.itemSizeSmall
            onClicked: {
                inPlaceSidebar.canceled()
            }
        }

        // --- Divider between Action Group and Heading/List Group ---
        Item {
            width: parent.width
            height: Theme.paddingSmall

            Rectangle {
                anchors.centerIn: parent
                width: parent.width - Theme.paddingMedium * 2
                height: 1
                color: Theme.rgba(Theme.highlightColor, 0.35)
            }
        }

        // --- Headings Group (H1, H2, H3, H4) ---
        BackgroundItem {
            width: parent.width
            height: Math.round(Theme.itemSizeExtraSmall * 0.8)
            anchors.horizontalCenter: parent.horizontalCenter
            onClicked: inPlaceSidebar.prefixRequested("= ", false)

            Label {
                anchors.centerIn: parent
                text: "H1"
                font.pixelSize: Theme.fontSizeExtraSmall
                font.bold: true
                color: parent.highlighted ? Theme.highlightColor : Theme.primaryColor
            }
        }

        BackgroundItem {
            width: parent.width
            height: Math.round(Theme.itemSizeExtraSmall * 0.8)
            anchors.horizontalCenter: parent.horizontalCenter
            onClicked: inPlaceSidebar.prefixRequested("== ", false)

            Label {
                anchors.centerIn: parent
                text: "H2"
                font.pixelSize: Theme.fontSizeExtraSmall
                font.bold: true
                color: parent.highlighted ? Theme.highlightColor : Theme.primaryColor
            }
        }

        BackgroundItem {
            width: parent.width
            height: Math.round(Theme.itemSizeExtraSmall * 0.8)
            anchors.horizontalCenter: parent.horizontalCenter
            onClicked: inPlaceSidebar.prefixRequested("=== ", false)

            Label {
                anchors.centerIn: parent
                text: "H3"
                font.pixelSize: Theme.fontSizeExtraSmall
                font.bold: true
                color: parent.highlighted ? Theme.highlightColor : Theme.primaryColor
            }
        }

        BackgroundItem {
            width: parent.width
            height: Math.round(Theme.itemSizeExtraSmall * 0.8)
            anchors.horizontalCenter: parent.horizontalCenter
            onClicked: inPlaceSidebar.prefixRequested("==== ", false)

            Label {
                anchors.centerIn: parent
                text: "H4"
                font.pixelSize: Theme.fontSizeExtraSmall
                font.bold: true
                color: parent.highlighted ? Theme.highlightColor : Theme.primaryColor
            }
        }

        // --- Spacing before Lists ---
        Item {
            width: parent.width
            height: 2
        }

        // --- Lists Group (Task Checkbox, Bullet, Numbered) ---
        BackgroundItem {
            width: parent.width
            height: Math.round(Theme.itemSizeExtraSmall * 0.8)
            anchors.horizontalCenter: parent.horizontalCenter
            onClicked: inPlaceSidebar.prefixRequested("* [ ] ", true)

            Image {
                anchors.centerIn: parent
                source: "image://theme/icon-s-task?" + (parent.highlighted ? Theme.highlightColor : Theme.primaryColor)
                width: Theme.iconSizeSmall * 0.8
                height: Theme.iconSizeSmall * 0.8
            }
        }

        BackgroundItem {
            width: parent.width
            height: Math.round(Theme.itemSizeExtraSmall * 0.8)
            anchors.horizontalCenter: parent.horizontalCenter
            onClicked: inPlaceSidebar.prefixRequested("* ", true)

            Label {
                anchors.centerIn: parent
                text: "•"
                font.pixelSize: Theme.fontSizeMedium
                font.bold: true
                color: parent.highlighted ? Theme.highlightColor : Theme.primaryColor
            }
        }

        BackgroundItem {
            width: parent.width
            height: Math.round(Theme.itemSizeExtraSmall * 0.8)
            anchors.horizontalCenter: parent.horizontalCenter
            onClicked: inPlaceSidebar.prefixRequested(". ", true)

            Label {
                anchors.centerIn: parent
                text: "1."
                font.pixelSize: Theme.fontSizeExtraSmall
                font.bold: true
                color: parent.highlighted ? Theme.highlightColor : Theme.primaryColor
            }
        }

        // --- Spacing before Link ---
        Item {
            width: parent.width
            height: 2
        }

        BackgroundItem {
            width: parent.width
            height: Math.round(Theme.itemSizeExtraSmall * 0.8)
            anchors.horizontalCenter: parent.horizontalCenter
            onClicked: inPlaceSidebar.linkRequested()

            Image {
                anchors.centerIn: parent
                source: "image://theme/icon-m-link?" + (parent.highlighted ? Theme.highlightColor : Theme.primaryColor)
                width: Theme.iconSizeSmall * 0.8
                height: Theme.iconSizeSmall * 0.8
            }
        }

        // --- Spacing before Paste ---
        Item {
            width: parent.width
            height: 2
        }

        BackgroundItem {
            width: parent.width
            height: Math.round(Theme.itemSizeExtraSmall * 0.8)
            anchors.horizontalCenter: parent.horizontalCenter
            onClicked: inPlaceSidebar.pasteRequested()

            Label {
                anchors.centerIn: parent
                text: "📋"
                font.pixelSize: Theme.fontSizeExtraSmall
                color: parent.highlighted ? Theme.highlightColor : Theme.primaryColor
            }
        }
    }
}
