import QtQuick 2.6
import Sailfish.Silica 1.0

Rectangle {
    id: bannerRoot
    width: parent ? Math.min(parent.width - Theme.horizontalPageMargin * 2, Theme.itemSizeHuge * 3) : Screen.width - Theme.horizontalPageMargin * 2
    height: bannerColumn.height + Theme.paddingMedium * 2
    anchors.horizontalCenter: parent ? parent.horizontalCenter : undefined
    z: 50

    property string message: qsTr("Discard unsaved changes?")
    property string discardText: qsTr("Discard")
    property string keepText: qsTr("Keep")
    property bool open: true

    signal discardConfirmed()
    signal keepEditing()

    radius: Theme.paddingMedium
    color: Theme.rgba(Theme.overlayBackgroundColor, 0.95)
    border.color: Theme.highlightColor
    border.width: 1
    clip: true

    opacity: open ? 1.0 : 0.0
    visible: opacity > 0.001
    enabled: open && visible

    Behavior on opacity {
        FadeAnimation { duration: 180 }
    }

    // Absorb clicks so interaction does not bleed through to underlying content
    MouseArea {
        anchors.fill: parent
        preventStealing: true
    }

    Column {
        id: bannerColumn
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.margins: Theme.paddingMedium
        spacing: Theme.paddingMedium

        Row {
            anchors.horizontalCenter: parent.horizontalCenter
            spacing: Theme.paddingSmall

            Icon {
                source: "image://theme/icon-m-warning"
                anchors.verticalCenter: parent.verticalCenter
                width: Theme.iconSizeSmall
                height: Theme.iconSizeSmall
            }

            Label {
                text: bannerRoot.message
                font.pixelSize: Theme.fontSizeSmall
                font.bold: true
                color: Theme.primaryColor
                anchors.verticalCenter: parent.verticalCenter
                truncationMode: TruncationMode.Fade
            }
        }

        Row {
            anchors.horizontalCenter: parent.horizontalCenter
            spacing: Theme.paddingLarge

            Button {
                text: bannerRoot.discardText
                preferredWidth: Theme.buttonWidthExtraSmall
                color: Theme.highlightColor
                onClicked: {
                    bannerRoot.discardConfirmed()
                }
            }

            Button {
                text: bannerRoot.keepText
                preferredWidth: Theme.buttonWidthExtraSmall
                onClicked: {
                    bannerRoot.keepEditing()
                }
            }
        }
    }
}
