import QtQuick 2.6
import Sailfish.Silica 1.0
import harbour.fishdoc 1.0
import "pages"
import "cover"

ApplicationWindow {
    id: app
    _defaultPageOrientations: Orientation.All

    Component.onCompleted: {
        pageStack.forceActiveFocus()
        bridge.load_main_page_data()
    }

    Timer {
        id: pollTimer
        interval: 50
        running: bridge.is_loading
        repeat: true
        onTriggered: bridge.poll_results()
    }

    Connections {
        target: RootApp
        onLastWindowClosed: Qt.quit()
    }

    FishdocBridge {
        id: bridge

        onError_occurred: {
            notification.text = message
            notification.show()
        }
    }

    initialPage: Component { MainPage {} }
    cover: Component { CoverPage {} }

    Rectangle {
        id: notification
        property alias text: notificationLabel.text

        function show() {
            notification.opacity = 1.0
            hideTimer.start()
        }

        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: parent.right
        height: Theme.itemSizeMedium
        color: Theme.highlightBackgroundColor
        opacity: 0.0
        z: 100

        Behavior on opacity { FadeAnimator {} }

        Label {
            id: notificationLabel
            anchors.centerIn: parent
            color: Theme.primaryColor
            font.pixelSize: Theme.fontSizeMedium
        }

        Timer {
            id: hideTimer
            interval: 3000
            onTriggered: notification.opacity = 0.0
        }

        MouseArea {
            anchors.fill: parent
            enabled: notification.opacity > 0
            onClicked: notification.opacity = 0.0
        }
    }
}
