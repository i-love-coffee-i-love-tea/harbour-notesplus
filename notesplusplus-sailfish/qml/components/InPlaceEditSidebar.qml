import QtQuick 2.6
import Sailfish.Silica 1.0

Item {
    id: inPlaceSidebar
    width: Theme.itemSizeSmall
    height: sidebarCol.height + Theme.paddingSmall * 2
    z: 30

    property bool specialPasteMode: false

    signal accepted()
    signal canceled()
    signal prefixRequested(string prefix, bool multiLine)
    signal linkRequested()
    signal pasteRequested()
    signal pasteSpecialRequested(string prefix, bool multiLine)
    signal refocusRequested()
    signal elementPickerRequested()

    function enterSpecialPasteMode() {
        specialPasteMode = true
        flashAnimation.restart()
        autoResetTimer.restart()
        inPlaceSidebar.refocusRequested()
    }

    function exitSpecialPasteMode() {
        specialPasteMode = false
        autoResetTimer.stop()
        flashAnimation.stop()
        flashRect.opacity = 0.0
        inPlaceSidebar.refocusRequested()
    }

    function handlePrefixClick(prefix, multiLine) {
        if (specialPasteMode && multiLine) {
            inPlaceSidebar.pasteSpecialRequested(prefix, multiLine)
            exitSpecialPasteMode()
        } else {
            if (specialPasteMode) {
                exitSpecialPasteMode()
            }
            inPlaceSidebar.prefixRequested(prefix, multiLine)
        }
    }

    onVisibleChanged: {
        if (!visible) {
            exitSpecialPasteMode()
        }
    }

    Timer {
        id: autoResetTimer
        interval: 10000
        repeat: false
        onTriggered: {
            inPlaceSidebar.exitSpecialPasteMode()
        }
    }

    // Absorb all touches so they don't leak through to the SilicaListView beneath
    MouseArea {
        anchors.fill: parent
        preventStealing: true
    }

    Rectangle {
        id: bgRect
        anchors.fill: parent
        radius: Theme.paddingMedium
        color: inPlaceSidebar.specialPasteMode ? Theme.rgba(Theme.overlayBackgroundColor, 0.95) : Theme.rgba(Theme.overlayBackgroundColor, 0.90)
        border.color: inPlaceSidebar.specialPasteMode ? Theme.highlightColor : Theme.rgba(Theme.highlightColor, 0.3)
        border.width: inPlaceSidebar.specialPasteMode ? 2 : 1

        Behavior on color { ColorAnimation { duration: 200 } }
        Behavior on border.color { ColorAnimation { duration: 200 } }
    }

    Rectangle {
        id: flashRect
        anchors.fill: parent
        radius: Theme.paddingMedium
        color: Theme.highlightColor
        opacity: 0.0
        z: 1
        visible: opacity > 0.001
    }

    SequentialAnimation {
        id: flashAnimation
        NumberAnimation {
            target: flashRect
            property: "opacity"
            from: 0.0
            to: 0.45
            duration: 100
            easing.type: Easing.OutQuad
        }
        NumberAnimation {
            target: flashRect
            property: "opacity"
            from: 0.45
            to: 0.10
            duration: 120
            easing.type: Easing.InOutQuad
        }
        NumberAnimation {
            target: flashRect
            property: "opacity"
            from: 0.10
            to: 0.40
            duration: 100
            easing.type: Easing.InOutQuad
        }
        NumberAnimation {
            target: flashRect
            property: "opacity"
            from: 0.40
            to: 0.0
            duration: 180
            easing.type: Easing.InQuad
        }
    }

    Column {
        id: sidebarCol
        anchors.centerIn: parent
        width: parent.width
        spacing: 2
        z: 5

        // --- Action Group (Check / Cancel) ---
        IconButton {
            id: acceptBtn
            icon.source: "image://theme/icon-m-accept"
            icon.width: Theme.iconSizeSmall + 4
            icon.height: Theme.iconSizeSmall + 4
            anchors.horizontalCenter: parent.horizontalCenter
            height: Theme.itemSizeExtraSmall
            width: Theme.itemSizeSmall
            onClicked: {
                if (inPlaceSidebar.specialPasteMode) {
                    inPlaceSidebar.exitSpecialPasteMode()
                }
                inPlaceSidebar.accepted()
            }
        }

        IconButton {
            id: cancelBtn
            icon.source: "image://theme/icon-m-clear"
            icon.width: Theme.iconSizeSmall + 4
            icon.height: Theme.iconSizeSmall + 4
            anchors.horizontalCenter: parent.horizontalCenter
            height: Theme.itemSizeExtraSmall
            width: Theme.itemSizeSmall
            onClicked: {
                if (inPlaceSidebar.specialPasteMode) {
                    inPlaceSidebar.exitSpecialPasteMode()
                    return
                }
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
                color: inPlaceSidebar.specialPasteMode ? Theme.rgba(Theme.highlightColor, 0.6) : Theme.rgba(Theme.highlightColor, 0.35)
            }
        }

        // --- Headings Group (H1, H2, H3) ---
        BackgroundItem {
            width: parent.width
            height: Math.round(Theme.itemSizeExtraSmall * 0.8)
            anchors.horizontalCenter: parent.horizontalCenter
            highlighted: down
            onClicked: inPlaceSidebar.handlePrefixClick("= ", false)

            Label {
                anchors.centerIn: parent
                text: "H1"
                font.pixelSize: Theme.fontSizeExtraSmall + 2
                font.bold: true
                color: parent.down ? Theme.highlightColor : Theme.primaryColor
            }
        }

        BackgroundItem {
            width: parent.width
            height: Math.round(Theme.itemSizeExtraSmall * 0.8)
            anchors.horizontalCenter: parent.horizontalCenter
            highlighted: down
            onClicked: inPlaceSidebar.handlePrefixClick("== ", false)

            Label {
                anchors.centerIn: parent
                text: "H2"
                font.pixelSize: Theme.fontSizeExtraSmall + 2
                font.bold: true
                color: parent.down ? Theme.highlightColor : Theme.primaryColor
            }
        }

        BackgroundItem {
            width: parent.width
            height: Math.round(Theme.itemSizeExtraSmall * 0.8)
            anchors.horizontalCenter: parent.horizontalCenter
            highlighted: down
            onClicked: inPlaceSidebar.handlePrefixClick("=== ", false)

            Label {
                anchors.centerIn: parent
                text: "H3"
                font.pixelSize: Theme.fontSizeExtraSmall + 2
                font.bold: true
                color: parent.down ? Theme.highlightColor : Theme.primaryColor
            }
        }

        // --- Spacing before Lists ---
        Item {
            width: parent.width
            height: 2
        }

        // --- Lists Group (Task Checkbox, Bullet, Numbered) ---
        BackgroundItem {
            id: checkboxItem
            width: parent.width
            height: Math.round(Theme.itemSizeExtraSmall * 0.8)
            anchors.horizontalCenter: parent.horizontalCenter
            highlighted: down || inPlaceSidebar.specialPasteMode
            onClicked: inPlaceSidebar.handlePrefixClick("* [ ] ", true)

            Rectangle {
                anchors.centerIn: parent
                width: Math.round(Theme.iconSizeSmall * 0.6) + 4
                height: Math.round(Theme.iconSizeSmall * 0.6) + 4
                color: "transparent"
                border.width: 2
                border.color: (checkboxItem.down || inPlaceSidebar.specialPasteMode) ? Theme.highlightColor : Theme.primaryColor
                radius: 3
            }
        }

        BackgroundItem {
            id: bulletListItem
            width: parent.width
            height: Math.round(Theme.itemSizeExtraSmall * 0.8)
            anchors.horizontalCenter: parent.horizontalCenter
            highlighted: down || inPlaceSidebar.specialPasteMode
            onClicked: inPlaceSidebar.handlePrefixClick("* ", true)

            Item {
                id: listIconContainer
                anchors.centerIn: parent
                width: Math.round(Theme.iconSizeSmall * 0.8) + 4
                height: Math.round(Theme.iconSizeSmall * 0.8) + 4

                Column {
                    anchors.centerIn: parent
                    spacing: 3

                    Repeater {
                        model: 3
                        Row {
                            spacing: 3
                            Rectangle {
                                width: 3
                                height: 3
                                radius: 1.5
                                color: (bulletListItem.down || inPlaceSidebar.specialPasteMode) ? Theme.highlightColor : Theme.primaryColor
                                anchors.verticalCenter: parent.verticalCenter
                            }
                            Rectangle {
                                width: listIconContainer.width - 3 - 3 - 4
                                height: 2
                                radius: 1
                                color: (bulletListItem.down || inPlaceSidebar.specialPasteMode) ? Theme.highlightColor : Theme.primaryColor
                                anchors.verticalCenter: parent.verticalCenter
                            }
                        }
                    }
                }
            }
        }

        BackgroundItem {
            id: numberedListItem
            width: parent.width
            height: Math.round(Theme.itemSizeExtraSmall * 0.8)
            anchors.horizontalCenter: parent.horizontalCenter
            highlighted: down || inPlaceSidebar.specialPasteMode
            onClicked: inPlaceSidebar.handlePrefixClick(". ", true)

            Label {
                anchors.centerIn: parent
                text: "1."
                font.pixelSize: Theme.fontSizeExtraSmall + 2
                font.bold: true
                color: (numberedListItem.down || inPlaceSidebar.specialPasteMode) ? Theme.highlightColor : Theme.primaryColor
            }
        }

        // --- Spacing before Link ---
        Item {
            width: parent.width
            height: 2
        }

        BackgroundItem {
            id: linkBtn
            width: parent.width
            height: Math.round(Theme.itemSizeExtraSmall * 0.8)
            anchors.horizontalCenter: parent.horizontalCenter
            highlighted: down
            onClicked: {
                if (inPlaceSidebar.specialPasteMode) {
                    inPlaceSidebar.exitSpecialPasteMode()
                }
                inPlaceSidebar.linkRequested()
            }

            Image {
                anchors.centerIn: parent
                source: "image://theme/icon-m-link?" + (linkBtn.down ? Theme.highlightColor : Theme.primaryColor)
                width: Math.round(Theme.iconSizeSmall * 0.8) + 4
                height: Math.round(Theme.iconSizeSmall * 0.8) + 4
            }
        }

        // --- Spacing before Paste ---
        Item {
            width: parent.width
            height: 2
        }

        BackgroundItem {
            id: pasteBtn
            width: parent.width
            height: Math.round(Theme.itemSizeExtraSmall * 0.8)
            anchors.horizontalCenter: parent.horizontalCenter
            highlighted: down

            property bool holdTriggered: false

            Timer {
                id: holdTimer
                interval: 500
                repeat: false
                onTriggered: {
                    pasteBtn.holdTriggered = true
                    inPlaceSidebar.enterSpecialPasteMode()
                }
            }

            onDownChanged: {
                if (down) {
                    pasteBtn.holdTriggered = false
                    holdTimer.restart()
                } else {
                    holdTimer.stop()
                    if (pasteBtn.holdTriggered || inPlaceSidebar.specialPasteMode) {
                        inPlaceSidebar.refocusRequested()
                    }
                }
            }

            onCanceled: {
                holdTimer.stop()
                if (pasteBtn.holdTriggered || inPlaceSidebar.specialPasteMode) {
                    inPlaceSidebar.refocusRequested()
                }
            }

            onClicked: {
                if (pasteBtn.holdTriggered) {
                    pasteBtn.holdTriggered = false
                    inPlaceSidebar.refocusRequested()
                    return
                }
                if (inPlaceSidebar.specialPasteMode) {
                    inPlaceSidebar.pasteRequested()
                    inPlaceSidebar.exitSpecialPasteMode()
                } else {
                    inPlaceSidebar.pasteRequested()
                }
            }

            Label {
                anchors.centerIn: parent
                text: "\ud83d\udccb"
                font.pixelSize: Theme.fontSizeSmall
                color: pasteBtn.down ? Theme.highlightColor : Theme.primaryColor
            }
        }

        // --- Spacing before Element Picker ---
        Item {
            width: parent.width
            height: 2
        }

        BackgroundItem {
            id: elementPickerBtn
            width: parent.width
            height: Math.round(Theme.itemSizeExtraSmall * 0.8)
            anchors.horizontalCenter: parent.horizontalCenter
            highlighted: down
            onClicked: {
                if (inPlaceSidebar.specialPasteMode) {
                    inPlaceSidebar.exitSpecialPasteMode()
                }
                inPlaceSidebar.elementPickerRequested()
            }

            Label {
                anchors.centerIn: parent
                text: "\u2026"
                font.pixelSize: Theme.fontSizeMedium
                color: elementPickerBtn.down ? Theme.highlightColor : Theme.primaryColor
            }
        }
    }
}
