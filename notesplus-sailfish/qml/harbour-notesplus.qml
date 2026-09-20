import QtQuick 2.6
import Sailfish.Silica 1.0
import harbour.notesplus 1.0
import "pages"
import "cover"
import "components/common"

ApplicationWindow {
    id: app
    _defaultPageOrientations: Orientation.All

    AppSettings {
        id: appSettings
    }

    // Single source of truth for default AI endpoint — referenced everywhere
    readonly property string defaultAiEndpoint: appSettings.defaultAiEndpoint

    property alias fontScale: appSettings.fontScale
    property alias docFontFamily: appSettings.docFontFamily
    property alias codeFontScale: appSettings.codeFontScale
    property alias tocCollapseThreshold: appSettings.tocCollapseThreshold
    property alias previewScale: appSettings.previewScale
    property alias gridColumns: appSettings.gridColumns
    property alias dropComments: appSettings.dropComments
    property alias allowExternalImages: appSettings.allowExternalImages
    property alias autostartWebServer: appSettings.autostartWebServer
    property alias rejectPublicNetworks: appSettings.rejectPublicNetworks
    property alias bindAddress: appSettings.bindAddress
    property alias sessionExpiryHours: appSettings.sessionExpiryHours
    property alias groupDisplayDepth: appSettings.groupDisplayDepth
    property alias journalEnabled: appSettings.journalEnabled
    property alias notesPath: appSettings.notesPath
    property alias notification: notification

    function formatSize(bytes) {
        if (!bytes || bytes <= 0) return ""
        var mb = bytes / (1024 * 1024)
        return mb.toFixed(0) + " MB"
    }

    function resolvedFontFamily() {
        return (docFontFamily && docFontFamily.length > 0) ? docFontFamily : Theme.fontFamily
    }

    function scaledFontSize(base) {
        return Math.round(base * fontScale)
    }

    property alias aiEnabled: appSettings.aiEnabled
    property alias sttEnabled: appSettings.sttEnabled
    property alias sttModel: appSettings.sttModel

    property alias aiProvider: appSettings.aiProvider
    property alias aiEndpoint: appSettings.aiEndpoint
    property alias aiModel: appSettings.aiModel
    property alias aiApiKey: appSettings.aiApiKey
    property alias aiTimeout: appSettings.aiTimeout
    property alias aiAutoAllowRead: appSettings.aiAutoAllowRead
    property alias aiAutoAllowCreate: appSettings.aiAutoAllowCreate
    property alias aiRequireConfirmEdit: appSettings.aiRequireConfirmEdit
    property alias aiAllowFetchUrl: appSettings.aiAllowFetchUrl
    property alias aiAllowSelfSigned: appSettings.aiAllowSelfSigned
    property alias aiSystemPrompt: appSettings.aiSystemPrompt

    property alias defaultCustomAiInstructions: appSettings.defaultCustomAiInstructions
    property alias customAiInstructions: appSettings.customAiInstructions

    function isDefaultAiInstruction(id) { return appSettings.isDefaultAiInstruction(id) }
    function getDefaultAiInstruction(id) { return appSettings.getDefaultAiInstruction(id) }

    function saveCustomAiInstruction(item) { appSettings.saveCustomAiInstruction(item) }
    function deleteCustomAiInstruction(id) { appSettings.deleteCustomAiInstruction(id) }
    function resetSingleAiInstruction(id) { appSettings.resetSingleAiInstruction(id) }
    function resetCustomAiInstructions() { appSettings.resetCustomAiInstructions() }

    function setFontScale(scale) {
        appSettings.setFontScale(scale)
    }

    function setFontFamily(family) {
        appSettings.setFontFamily(family)
    }

    function setCodeFontScale(scale) {
        appSettings.setCodeFontScale(scale)
    }

    function setTocCollapseThreshold(threshold) {
        appSettings.setTocCollapseThreshold(threshold)
    }

    function setPreviewScale(scale) {
        appSettings.setPreviewScale(scale)
    }

    function setGridColumns(cols) {
        appSettings.setGridColumns(cols)
    }

    function setDropComments(drop) {
        appSettings.setDropComments(drop)
        bridge.set_drop_comments(drop)
    }

    function setAllowExternalImages(allow) {
        appSettings.setAllowExternalImages(allow)
    }

    function setAutostartWebServer(val) {
        appSettings.setAutostartWebServer(val)
    }

    function setRejectPublicNetworks(val) {
        appSettings.setRejectPublicNetworks(val)
        bridge.set_reject_public_networks(val)
    }

    function setBindAddress(addr) {
        appSettings.setBindAddress(addr)
        bridge.set_bind_address(addr)
    }

    function setSessionExpiryHours(hours) {
        appSettings.setSessionExpiryHours(hours)
        if (typeof bridge !== "undefined" && bridge && typeof bridge.set_session_expiry_hours === "function") {
            bridge.set_session_expiry_hours(hours)
        }
    }

    function setJournalEnabled(val) {
        appSettings.setJournalEnabled(val)
    }

    function setNotesPath(path) {
        appSettings.setNotesPath(path)
    }

    function setGroupDisplayDepth(val) {
        appSettings.setGroupDisplayDepth(val)
        bridge.set_group_display_depth(val)
    }

    function setAiEnabled(val) {
        appSettings.setAiEnabled(val)
    }

    function syncAiConfig() {
        if (typeof bridge !== "undefined" && bridge && typeof bridge.configure_ai === "function") {
            bridge.configure_ai(
                app.aiProvider || "ollama",
                app.aiEndpoint || app.defaultAiEndpoint,
                app.aiModel || "llama3.2",
                app.aiApiKey || "",
                app.aiTimeout || 90,
                app.aiAutoAllowRead !== undefined ? app.aiAutoAllowRead : true,
                app.aiAutoAllowCreate !== undefined ? app.aiAutoAllowCreate : true,
                app.aiRequireConfirmEdit !== undefined ? app.aiRequireConfirmEdit : true,
                app.aiAllowSelfSigned !== undefined ? app.aiAllowSelfSigned : false,
                app.aiAllowFetchUrl !== undefined ? app.aiAllowFetchUrl : true
            )
        }
    }

    function syncTheme() {
        if (typeof bridge !== "undefined" && bridge && typeof bridge.set_theme === "function") {
            var isDark = (Theme.colorScheme !== Theme.DarkOnLight)
            var scheme = isDark ? "dark" : "light"
            var highlight = String(Theme.highlightColor)
            var primary = String(Theme.primaryColor)
            var secondary = String(Theme.secondaryColor)
            var highlightBg = String(Theme.highlightBackgroundColor)
            var colors = {
                "colorScheme": scheme,
                "primaryColor": primary,
                "secondaryColor": secondary,
                "highlightColor": highlight,
                "highlightBackgroundColor": highlightBg,
                "primary": highlight,
                "primary-hover": highlight,
                "accent": highlight,
                "accent-light": highlightBg
            }
            bridge.set_theme(JSON.stringify(colors))
        }
    }

    function setAiProvider(provider) {
        appSettings.setAiProvider(provider)
        syncAiConfig()
    }

    function setAiEndpoint(endpoint) {
        appSettings.setAiEndpoint(endpoint)
        syncAiConfig()
    }

    function setAiModel(model) {
        appSettings.setAiModel(model)
        syncAiConfig()
    }

    function setAiApiKey(key) {
        appSettings.setAiApiKey(key)
        syncAiConfig()
    }

    function setAiTimeout(secs) {
        appSettings.setAiTimeout(secs)
        syncAiConfig()
    }

    function setAiAutoAllowRead(val) {
        appSettings.setAiAutoAllowRead(val)
        syncAiConfig()
    }

    function setAiAutoAllowCreate(val) {
        appSettings.setAiAutoAllowCreate(val)
        syncAiConfig()
    }

    function setAiRequireConfirmEdit(val) {
        appSettings.setAiRequireConfirmEdit(val)
        syncAiConfig()
    }

    function setAiAllowFetchUrl(val) {
        appSettings.setAiAllowFetchUrl(val)
        syncAiConfig()
    }

    function setAiAllowSelfSigned(val) {
        appSettings.setAiAllowSelfSigned(val)
        syncAiConfig()
    }

    function setAiSystemPrompt(val) {
        appSettings.setAiSystemPrompt(val)
    }

    function setSttEnabled(val) {
        appSettings.setSttEnabled(val)
    }

    function setSttModel(modelId) {
        appSettings.setSttModel(modelId)
        if (typeof speechBridge !== "undefined" && speechBridge && typeof speechBridge.set_active_model === "function") {
            speechBridge.set_active_model(modelId)
        }
    }

    function openAssistant(contextFilename, contextContent, extraContext, attachedNotes) {
        var props = {
            contextFilename: contextFilename || "",
            contextContent: contextContent || "",
            extraContext: extraContext || ""
        }
        if (attachedNotes && attachedNotes.length > 0) {
            props.attachedNotes = attachedNotes
        }
        pageStack.push(Qt.resolvedUrl("pages/AssistantPage.qml"), props)
    }

    function openAiImport() {
        pageStack.push(Qt.resolvedUrl("pages/AiImportPage.qml"))
    }

    function openSearch() {
        pageStack.pop(null, PageStackAction.Immediate)
        bridge.load_main_page_data()
        app.activate()
        if (pageStack.currentPage && typeof pageStack.currentPage.activateSearch === "function") {
            pageStack.currentPage.activateSearch()
        }
    }

    function openJournal() {
        pageStack.pop(null, PageStackAction.Immediate)
        pageStack.push(Qt.resolvedUrl("pages/PageView.qml"), {
            pageName: "Journal"
        })
        bridge.load_page("Journal")
        app.activate()
    }

    Component.onCompleted: {
        syncAiConfig()
        if (app.sttModel && app.sttModel.length > 0 && typeof speechBridge !== "undefined" && speechBridge) {
            speechBridge.set_active_model(app.sttModel)
        }
        bridge.set_drop_comments(app.dropComments)
        bridge.set_reject_public_networks(app.rejectPublicNetworks)
        bridge.set_bind_address(app.bindAddress)
        bridge.set_session_expiry_hours(app.sessionExpiryHours)
        bridge.set_group_display_depth(app.groupDisplayDepth)
        if (app.notesPath && app.notesPath.length > 0) {
            bridge.set_notes_dir(app.notesPath)
        }
        syncTheme()
        if (app.autostartWebServer) {
            bridge.start_web_server()
            syncTheme()
        }
        pageStack.forceActiveFocus()
    }

    property var activeAuthPromptPage: null

    function showOrUpdateAuthPrompt(challengeId, verificationCode) {
        if (!challengeId) return
        if (activeAuthPromptPage) {
            if (typeof activeAuthPromptPage.updateChallenge === "function") {
                activeAuthPromptPage.updateChallenge(challengeId, verificationCode)
                return
            }
        }
        var page = pageStack.push(Qt.resolvedUrl("pages/AuthPrompt.qml"), {
            challengeId: challengeId,
            verificationCode: verificationCode
        })
        activeAuthPromptPage = page
    }

    Timer {
        id: authChallengePollTimer
        interval: 500
        repeat: true
        running: false
        onTriggered: {
            if (bridge.check_auth_challenge()) {
                showOrUpdateAuthPrompt(bridge.auth_challenge_id, bridge.auth_verification_code)
            }
        }
    }

    Connections {
        target: RootApp
        onLastWindowClosed: Qt.quit()
    }

    Connections {
        target: Theme
        onHighlightColorChanged: syncTheme()
    }

    NotesBridge {
        id: bridge

        onError_occurred: {
            notification.text = message
            notification.show()
        }
        onWeb_server_status_changed: {
            if (bridge.web_server_running) {
                syncTheme()
                authChallengePollTimer.start()
            } else {
                authChallengePollTimer.stop()
            }
        }
        onAuth_challenge_changed: {
            if (bridge.auth_challenge_pending) {
                showOrUpdateAuthPrompt(bridge.auth_challenge_id, bridge.auth_verification_code)
            }
        }
        onInitialized_changed: {
            if (bridge.initialized) {
                syncTheme()
            }
        }
        onNotes_dir_changed: {
            if (app.notesPath !== bridge.notes_dir) {
                app.setNotesPath(bridge.notes_dir)
            }
        }
    }

    SpeechBridge {
        id: speechBridge

        onActive_model_changed: {
            if (active_model_id && active_model_id.length > 0 && active_model_id !== app.sttModel) {
                sttModelConf.value = active_model_id
            }
        }
    }

    AgentBridge {
        id: agentBridge
    }

    initialPage: Component { MainPage {} }
    cover: Component { CoverPage {} }

    Rectangle {
        id: notification
        property alias text: notificationLabel.text
        property string targetFilePath: ""

        function show(msg, filePath) {
            if (typeof msg === "string" && msg.length > 0) {
                notificationLabel.text = msg
            }
            if (!notificationLabel.text || notificationLabel.text.length === 0) {
                return
            }
            targetFilePath = (typeof filePath === "string") ? filePath : ""
            notification.opacity = 1.0
            hideTimer.restart()
        }

        anchors.top: parent.top
        anchors.topMargin: Theme.paddingLarge * 2
        anchors.horizontalCenter: parent.horizontalCenter
        width: Math.min(parent.width - Theme.horizontalPageMargin * 2, contentCol.implicitWidth + Theme.paddingLarge * 2)
        height: Math.max(Theme.itemSizeExtraSmall, contentCol.implicitHeight + Theme.paddingSmall * 2)
        color: Qt.tint(
                   Theme.rgba(Theme.overlayBackgroundColor, Theme.opacityOverlay),
                   Theme.rgba(Theme.highlightBackgroundColor, Theme.highlightBackgroundOpacity))
        border.color: Theme.rgba(Theme.highlightColor, 0.4)
        border.width: 1
        radius: Theme.paddingSmall
        opacity: 0.0
        z: 1000

        Behavior on opacity { FadeAnimation {} }

        Column {
            id: contentCol
            anchors.centerIn: parent
            width: Math.min(notification.parent ? (notification.parent.width - Theme.horizontalPageMargin * 2 - Theme.paddingLarge * 2) : 300, implicitWidth)
            spacing: Theme.paddingSmall / 2

            Label {
                id: notificationLabel
                anchors.horizontalCenter: parent.horizontalCenter
                width: Math.min(notification.parent ? (notification.parent.width - Theme.horizontalPageMargin * 2 - Theme.paddingLarge * 2) : 300, implicitWidth)
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.Wrap
                color: Theme.primaryColor
                font.pixelSize: Theme.fontSizeExtraSmall
            }

            Label {
                id: hintLabel
                anchors.horizontalCenter: parent.horizontalCenter
                visible: notification.targetFilePath.length > 0
                text: qsTr("Tap to open")
                color: Theme.highlightColor
                font.pixelSize: Theme.fontSizeTiny
            }
        }

        Timer {
            id: hideTimer
            interval: notification.targetFilePath.length > 0 ? 5000 : 3500
            onTriggered: {
                notification.opacity = 0.0
                notification.targetFilePath = ""
            }
        }

        MouseArea {
            anchors.fill: parent
            enabled: notification.opacity > 0
            onClicked: {
                if (notification.targetFilePath && notification.targetFilePath.length > 0) {
                    var target = notification.targetFilePath
                    if (target.indexOf("file://") !== 0) {
                        target = "file://" + target
                    }
                    Qt.openUrlExternally(target)
                }
                notification.opacity = 0.0
                notification.targetFilePath = ""
            }
        }
    }
}
