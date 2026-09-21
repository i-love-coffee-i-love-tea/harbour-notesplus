import QtQuick 2.6
import Sailfish.Silica 1.0

Item {
    id: inPlaceSidebar
    width: Theme.itemSizeSmall
    height: sidebarCol.height + Theme.paddingSmall * 2
    z: 30

    property bool specialPasteMode: false
    property bool flyoutOpen: false
    property bool showVoiceInput: true
    readonly property bool isSpeechRecording: typeof speechBridge !== "undefined" && speechBridge && speechBridge.is_recording
    readonly property real liveAudioLevel: (typeof speechBridge !== "undefined" && speechBridge && isSpeechRecording) ? speechBridge.audio_level : 0.0

    signal accepted()
    signal canceled()
    signal voiceInputRequested()
    signal prefixRequested(string prefix, bool multiLine)
    signal linkRequested()
    signal pasteRequested()
    signal pasteSpecialRequested(string prefix, bool multiLine)
    signal refocusRequested()
    signal elementPickerRequested()
    signal indentRequested()
    signal outdentRequested()
    signal moveUpRequested()
    signal moveDownRequested()

    function openFlyout() {
        flyoutOpen = true
        flyoutTimer.restart()
    }

    function closeFlyout() {
        flyoutOpen = false
        flyoutTimer.stop()
    }

    function toggleFlyout() {
        if (flyoutOpen) {
            closeFlyout()
        } else {
            openFlyout()
        }
    }

    function enterSpecialPasteMode() {
        closeFlyout()
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
            closeFlyout()
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

    Timer {
        id: flyoutTimer
        interval: 8000
        repeat: false
        onTriggered: {
            inPlaceSidebar.closeFlyout()
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

        Item {
            id: micItem
            width: Theme.itemSizeSmall
            height: visible ? Theme.itemSizeExtraSmall : 0
            anchors.horizontalCenter: parent.horizontalCenter
            visible: inPlaceSidebar.showVoiceInput && (typeof app === "undefined" || !app || app.sttEnabled !== false)

            Rectangle {
                anchors.centerIn: parent
                width: Math.min(parent.width, Theme.iconSizeSmall + Theme.paddingSmall + Math.round(inPlaceSidebar.liveAudioLevel * 20))
                height: width
                radius: width / 2
                color: (inPlaceSidebar.liveAudioLevel > 0.06) ? Theme.highlightColor : Theme.secondaryColor
                opacity: inPlaceSidebar.isSpeechRecording ? Math.min(0.85, 0.25 + inPlaceSidebar.liveAudioLevel * 0.6) : 0.0
                visible: inPlaceSidebar.isSpeechRecording

                Behavior on width { NumberAnimation { duration: 60 } }
                Behavior on height { NumberAnimation { duration: 60 } }
                Behavior on opacity { NumberAnimation { duration: 60 } }
            }

            IconButton {
                id: micBtn
                icon.source: inPlaceSidebar.isSpeechRecording ? "image://theme/icon-m-clear" : "image://theme/icon-m-mic"
                icon.width: Theme.iconSizeSmall + 4
                icon.height: Theme.iconSizeSmall + 4
                anchors.centerIn: parent
                height: Theme.itemSizeExtraSmall
                width: Theme.itemSizeSmall
                highlighted: inPlaceSidebar.isSpeechRecording
                onClicked: {
                    if (inPlaceSidebar.specialPasteMode) {
                        inPlaceSidebar.exitSpecialPasteMode()
                    }
                    inPlaceSidebar.voiceInputRequested()
                }
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
        Column {
            id: listGroupContainer
            width: parent.width
            spacing: 2

            Timer {
                id: listHoldTimer
                interval: 400
                repeat: false
                onTriggered: {
                    inPlaceSidebar.openFlyout()
                }
            }

            BackgroundItem {
                id: checkboxItem
                width: parent.width
                height: Math.round(Theme.itemSizeExtraSmall * 0.8)
                anchors.horizontalCenter: parent.horizontalCenter
                highlighted: down || inPlaceSidebar.specialPasteMode
                onDownChanged: {
                    if (down) listHoldTimer.restart(); else listHoldTimer.stop();
                }
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
                highlighted: down || inPlaceSidebar.specialPasteMode || inPlaceSidebar.flyoutOpen
                onDownChanged: {
                    if (down) listHoldTimer.restart(); else listHoldTimer.stop();
                }
                onClicked: {
                    if (inPlaceSidebar.flyoutOpen) {
                        inPlaceSidebar.toggleFlyout()
                    } else {
                        inPlaceSidebar.handlePrefixClick("* ", true)
                    }
                }

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
                                    color: (bulletListItem.down || inPlaceSidebar.specialPasteMode || inPlaceSidebar.flyoutOpen) ? Theme.highlightColor : Theme.primaryColor
                                    anchors.verticalCenter: parent.verticalCenter
                                }
                                Rectangle {
                                    width: listIconContainer.width - 3 - 3 - 4
                                    height: 2
                                    radius: 1
                                    color: (bulletListItem.down || inPlaceSidebar.specialPasteMode || inPlaceSidebar.flyoutOpen) ? Theme.highlightColor : Theme.primaryColor
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
                onDownChanged: {
                    if (down) listHoldTimer.restart(); else listHoldTimer.stop();
                }
                onClicked: inPlaceSidebar.handlePrefixClick(". ", true)

                Label {
                    anchors.centerIn: parent
                    text: "1."
                    font.pixelSize: Theme.fontSizeExtraSmall + 2
                    font.bold: true
                    color: (numberedListItem.down || inPlaceSidebar.specialPasteMode) ? Theme.highlightColor : Theme.primaryColor
                }
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

    // --- Contextual Flyout Palette for List Operations ---
    Rectangle {
        id: flyoutPalette
        anchors.right: parent.left
        anchors.rightMargin: Theme.paddingSmall
        anchors.verticalCenter: listGroupContainer.verticalCenter
        width: flyoutRow.width + Theme.paddingSmall * 2
        height: Theme.itemSizeExtraSmall
        radius: Theme.paddingMedium
        color: Theme.rgba(Theme.overlayBackgroundColor, 0.92)
        border.color: Theme.rgba(Theme.highlightColor, 0.4)
        border.width: 1
        opacity: inPlaceSidebar.flyoutOpen ? 1.0 : 0.0
        visible: opacity > 0.001
        z: 40

        Behavior on opacity { NumberAnimation { duration: 150 } }

        Row {
            id: flyoutRow
            anchors.centerIn: parent
            spacing: 2

            BackgroundItem {
                id: flyoutOutdentBtn
                width: Theme.itemSizeExtraSmall
                height: Theme.itemSizeExtraSmall
                highlighted: down
                onClicked: {
                    inPlaceSidebar.openFlyout()
                    inPlaceSidebar.outdentRequested()
                    inPlaceSidebar.refocusRequested()
                }

                Label {
                    anchors.centerIn: parent
                    text: "⇤"
                    font.pixelSize: Theme.fontSizeMedium
                    font.bold: true
                    color: flyoutOutdentBtn.down ? Theme.highlightColor : Theme.primaryColor
                }
            }

            BackgroundItem {
                id: flyoutIndentBtn
                width: Theme.itemSizeExtraSmall
                height: Theme.itemSizeExtraSmall
                highlighted: down
                onClicked: {
                    inPlaceSidebar.openFlyout()
                    inPlaceSidebar.indentRequested()
                    inPlaceSidebar.refocusRequested()
                }

                Label {
                    anchors.centerIn: parent
                    text: "⇥"
                    font.pixelSize: Theme.fontSizeMedium
                    font.bold: true
                    color: flyoutIndentBtn.down ? Theme.highlightColor : Theme.primaryColor
                }
            }

            BackgroundItem {
                id: flyoutMoveUpBtn
                width: Theme.itemSizeExtraSmall
                height: Theme.itemSizeExtraSmall
                highlighted: down
                onClicked: {
                    inPlaceSidebar.openFlyout()
                    inPlaceSidebar.moveUpRequested()
                    inPlaceSidebar.refocusRequested()
                }

                Label {
                    anchors.centerIn: parent
                    text: "▲"
                    font.pixelSize: Theme.fontSizeSmall
                    color: flyoutMoveUpBtn.down ? Theme.highlightColor : Theme.primaryColor
                }
            }

            BackgroundItem {
                id: flyoutMoveDownBtn
                width: Theme.itemSizeExtraSmall
                height: Theme.itemSizeExtraSmall
                highlighted: down
                onClicked: {
                    inPlaceSidebar.openFlyout()
                    inPlaceSidebar.moveDownRequested()
                    inPlaceSidebar.refocusRequested()
                }

                Label {
                    anchors.centerIn: parent
                    text: "▼"
                    font.pixelSize: Theme.fontSizeSmall
                    color: flyoutMoveDownBtn.down ? Theme.highlightColor : Theme.primaryColor
                }
            }
        }
    }
}
