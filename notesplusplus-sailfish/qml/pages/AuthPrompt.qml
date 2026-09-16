import QtQuick 2.6
import Sailfish.Silica 1.0

Page {
    id: authPromptPage
    allowedOrientations: Orientation.All

    property string challengeId: ""
    property string verificationCode: ""
    property bool authHandled: false

    function updateChallenge(newChallengeId, newVerificationCode) {
        challengeId = newChallengeId
        verificationCode = newVerificationCode
        expireTimer.restart()
    }

    function acceptLogin() {
        if (authHandled) return
        authHandled = true
        if (challengeId && challengeId !== "test-challenge") {
            bridge.approve_auth_challenge(challengeId)
        }
        pageStack.pop()
    }

    function denyLogin() {
        if (authHandled) return
        authHandled = true
        if (challengeId && challengeId !== "test-challenge") {
            bridge.deny_auth_challenge(challengeId)
        }
        pageStack.pop()
    }

    Component.onDestruction: {
        if (typeof app !== "undefined" && app && app.activeAuthPromptPage === authPromptPage) {
            app.activeAuthPromptPage = null
        }
    }

    Timer {
        id: expireTimer
        interval: 60000
        running: true
        onTriggered: {
            denyLogin()
        }
    }

    Column {
        anchors.centerIn: parent
        width: parent.width - 2 * Theme.horizontalPageMargin
        spacing: Theme.paddingLarge

        Image {
            anchors.horizontalCenter: parent.horizontalCenter
            source: "image://theme/icon-m-device-lock"
            width: Theme.iconSizeExtraLarge
            height: Theme.iconSizeExtraLarge
            fillMode: Image.PreserveAspectFit
            opacity: 0.85
        }

        Label {
            anchors.horizontalCenter: parent.horizontalCenter
            width: parent.width
            horizontalAlignment: Text.AlignHCenter
            text: qsTr("Web UI Login Request")
            font.pixelSize: Theme.fontSizeLarge
            color: Theme.highlightColor
        }

        Label {
            anchors.horizontalCenter: parent.horizontalCenter
            width: parent.width
            horizontalAlignment: Text.AlignHCenter
            text: qsTr("A web browser is requesting access to Notes Plus.")
            font.pixelSize: Theme.fontSizeMedium
            color: Theme.primaryColor
            wrapMode: Text.WordWrap
        }

        Column {
            anchors.horizontalCenter: parent.horizontalCenter
            width: parent.width
            spacing: Theme.paddingSmall
            visible: verificationCode.length > 0

            Label {
                anchors.horizontalCenter: parent.horizontalCenter
                width: parent.width
                horizontalAlignment: Text.AlignHCenter
                text: qsTr("Verify this code matches the browser:")
                font.pixelSize: Theme.fontSizeSmall
                color: Theme.secondaryColor
            }

            Rectangle {
                anchors.horizontalCenter: parent.horizontalCenter
                width: codeLabel.width + Theme.paddingLarge * 2
                height: codeLabel.height + Theme.paddingMedium * 2
                radius: Theme.paddingSmall
                color: "transparent"
                border.color: Theme.highlightColor
                border.width: 2

                Label {
                    id: codeLabel
                    anchors.centerIn: parent
                    text: verificationCode
                    font.pixelSize: Theme.fontSizeExtraLarge * 1.5
                    font.weight: Font.Bold
                    font.family: "monospace"
                    color: Theme.highlightColor
                }
            }
        }

        Button {
            anchors.horizontalCenter: parent.horizontalCenter
            text: qsTr("Accept")
            preferredWidth: Theme.buttonWidthMedium
            onClicked: {
                acceptLogin()
            }
        }
    }

    Button {
        anchors {
            bottom: parent.bottom
            bottomMargin: Theme.paddingLarge * 2
            horizontalCenter: parent.horizontalCenter
        }
        text: qsTr("Cancel")
        preferredWidth: Theme.buttonWidthMedium
        onClicked: {
            denyLogin()
        }
    }
}
