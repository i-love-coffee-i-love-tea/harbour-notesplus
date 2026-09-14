import QtQuick 2.6
import Sailfish.Silica 1.0
import Nemo.Configuration 1.0
import harbour.notesplusplus 1.0
import "pages"
import "cover"

ApplicationWindow {
    id: app
    _defaultPageOrientations: Orientation.All

    // Single source of truth for default AI endpoint — referenced everywhere
    readonly property string defaultAiEndpoint: "http://localhost:11434"

    ConfigurationValue {
        id: fontSizeScaleConf
        key: "/apps/harbour-notesplusplus/font_size_scale"
        defaultValue: 1.0
    }

    ConfigurationValue {
        id: fontFamilyConf
        key: "/apps/harbour-notesplusplus/font_family"
        defaultValue: ""
    }

    ConfigurationValue {
        id: codeFontScaleConf
        key: "/apps/harbour-notesplusplus/code_font_scale"
        defaultValue: 1.0
    }

    ConfigurationValue {
        id: tocCollapseThresholdConf
        key: "/apps/harbour-notesplusplus/toc_collapse_threshold"
        defaultValue: 5
    }

    ConfigurationValue {
        id: previewScaleConf
        key: "/apps/harbour-notesplusplus/preview_scale"
        defaultValue: 0.52
    }

    ConfigurationValue {
        id: gridColumnsConf
        key: "/apps/harbour-notesplusplus/grid_columns"
        defaultValue: 2
    }

    ConfigurationValue {
        id: dropCommentsConf
        key: "/apps/harbour-notesplusplus/drop_comments"
        defaultValue: true
    }

    ConfigurationValue {
        id: allowExternalImagesConf
        key: "/apps/harbour-notesplusplus/allow_external_images"
        defaultValue: true
    }

    ConfigurationValue {
        id: autostartWebServerConf
        key: "/apps/harbour-notesplusplus/autostart_web_server"
        defaultValue: false
    }

    ConfigurationValue {
        id: rejectPublicNetworksConf
        key: "/apps/harbour-notesplusplus/reject_public_networks"
        defaultValue: true
    }

    ConfigurationValue {
        id: bindAddressConf
        key: "/apps/harbour-notesplusplus/bind_address"
        defaultValue: "0.0.0.0"
    }

    ConfigurationValue {
        id: sessionExpiryHoursConf
        key: "/apps/harbour-notesplusplus/session_expiry_hours"
        defaultValue: 24
    }

    ConfigurationValue {
        id: groupDisplayDepthConf
        key: "/apps/harbour-notesplusplus/group_display_depth"
        defaultValue: 2
    }

    ConfigurationValue {
        id: journalEnabledConf
        key: "/apps/harbour-notesplusplus/journal_enabled"
        defaultValue: true
    }

    ConfigurationValue {
        id: aiEnabledConf
        key: "/apps/harbour-notesplusplus/ai_enabled"
        defaultValue: true
    }

    ConfigurationValue {
        id: aiProviderConf
        key: "/apps/harbour-notesplusplus/ai_provider"
        defaultValue: "ollama"
    }

    ConfigurationValue {
        id: aiEndpointConf
        key: "/apps/harbour-notesplusplus/ai_endpoint"
        defaultValue: app.defaultAiEndpoint
    }

    ConfigurationValue {
        id: aiModelConf
        key: "/apps/harbour-notesplusplus/ai_model"
        defaultValue: "llama3.2"
    }

    ConfigurationValue {
        id: aiApiKeyConf
        key: "/apps/harbour-notesplusplus/ai_api_key"
        defaultValue: ""
    }

    ConfigurationValue {
        id: aiTimeoutConf
        key: "/apps/harbour-notesplusplus/ai_timeout_secs"
        defaultValue: 90
    }

    ConfigurationValue {
        id: aiAutoAllowReadConf
        key: "/apps/harbour-notesplusplus/ai_auto_allow_read"
        defaultValue: true
    }

    ConfigurationValue {
        id: aiAutoAllowCreateConf
        key: "/apps/harbour-notesplusplus/ai_auto_allow_create"
        defaultValue: true
    }

    ConfigurationValue {
        id: aiRequireConfirmEditConf
        key: "/apps/harbour-notesplusplus/ai_require_confirm_edit"
        defaultValue: true
    }

    ConfigurationValue {
        id: aiAllowFetchUrlConf
        key: "/apps/harbour-notesplusplus/ai_allow_fetch_url"
        defaultValue: true
    }

    ConfigurationValue {
        id: aiAllowSelfSignedConf
        key: "/apps/harbour-notesplusplus/ai_allow_self_signed"
        defaultValue: false
    }

    ConfigurationValue {
        id: customAiInstructionsConf
        key: "/apps/harbour-notesplusplus/custom_ai_instructions"
        defaultValue: ""
    }

    ConfigurationValue {
        id: sttEnabledConf
        key: "/apps/harbour-notesplusplus/stt_enabled"
        defaultValue: true
    }

    ConfigurationValue {
        id: sttModelConf
        key: "/apps/harbour-notesplusplus/stt_model"
        defaultValue: ""
    }

    property real fontScale: fontSizeScaleConf.value !== undefined && fontSizeScaleConf.value > 0 ? fontSizeScaleConf.value : 1.0
    property string docFontFamily: fontFamilyConf.value !== undefined ? fontFamilyConf.value : ""
    property real codeFontScale: codeFontScaleConf.value !== undefined && codeFontScaleConf.value > 0 ? codeFontScaleConf.value : 1.0
    property int tocCollapseThreshold: tocCollapseThresholdConf.value !== undefined ? tocCollapseThresholdConf.value : 5
    property real previewScale: previewScaleConf.value !== undefined && previewScaleConf.value > 0 ? previewScaleConf.value : 0.52
    property int gridColumns: gridColumnsConf.value !== undefined ? gridColumnsConf.value : 2
    property bool dropComments: dropCommentsConf.value !== undefined ? dropCommentsConf.value : true
    property bool allowExternalImages: allowExternalImagesConf.value !== undefined ? allowExternalImagesConf.value : true
    property bool autostartWebServer: autostartWebServerConf.value !== undefined ? autostartWebServerConf.value : false
    property bool rejectPublicNetworks: rejectPublicNetworksConf.value !== undefined ? rejectPublicNetworksConf.value : true
    property string bindAddress: bindAddressConf.value !== undefined ? bindAddressConf.value : "0.0.0.0"
    property int sessionExpiryHours: sessionExpiryHoursConf.value !== undefined ? sessionExpiryHoursConf.value : 24
    property int groupDisplayDepth: groupDisplayDepthConf.value !== undefined ? groupDisplayDepthConf.value : 2
    property bool journalEnabled: journalEnabledConf.value !== undefined ? journalEnabledConf.value : true

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
    property bool aiEnabled: aiEnabledConf.value !== undefined ? aiEnabledConf.value : true
    property bool sttEnabled: sttEnabledConf.value !== undefined ? sttEnabledConf.value : true
    property string sttModel: sttModelConf.value !== undefined ? sttModelConf.value : ""

    property string aiProvider: aiProviderConf.value !== undefined ? aiProviderConf.value : "ollama"
    property string aiEndpoint: aiEndpointConf.value !== undefined ? aiEndpointConf.value : app.defaultAiEndpoint
    property string aiModel: aiModelConf.value !== undefined ? aiModelConf.value : "llama3.2"
    property string aiApiKey: aiApiKeyConf.value !== undefined ? aiApiKeyConf.value : ""
    property int aiTimeout: aiTimeoutConf.value !== undefined ? aiTimeoutConf.value : 90
    property bool aiAutoAllowRead: aiAutoAllowReadConf.value !== undefined ? aiAutoAllowReadConf.value : true
    property bool aiAutoAllowCreate: aiAutoAllowCreateConf.value !== undefined ? aiAutoAllowCreateConf.value : true
    property bool aiRequireConfirmEdit: aiRequireConfirmEditConf.value !== undefined ? aiRequireConfirmEditConf.value : true
    property bool aiAllowFetchUrl: aiAllowFetchUrlConf.value !== undefined ? aiAllowFetchUrlConf.value : true
    property bool aiAllowSelfSigned: aiAllowSelfSignedConf.value !== undefined ? aiAllowSelfSignedConf.value : false

    readonly property var defaultCustomAiInstructions: [
        {
            "id": "beautify",
            "buttonText": qsTr("Beautify"),
            "icon": "icon-m-favorite",
            "instruction": "Please beautify the active note by adding visual structure, helpful admonition blocks (NOTE, TIP, WARNING), clean tables, and suitable emoji accents where appropriate. Call the edit_note tool with the complete beautified AsciiDoc content and filename."
        },
        {
            "id": "extract_todos",
            "buttonText": qsTr("Extract To-Dos"),
            "icon": "icon-m-select-all",
            "instruction": "Please analyze the active note and extract all actionable tasks and todo items into a clean AsciiDoc checklist using `* [ ]`."
        },
        {
            "id": "fix_grammar",
            "buttonText": qsTr("Fix Grammar"),
            "icon": "icon-m-edit",
            "instruction": "Please review and correct the spelling, grammar, punctuation, and formatting in the active note while strictly preserving and enforcing proper AsciiDoc syntax. Call the edit_note tool with the complete corrected AsciiDoc content and filename."
        },
        {
            "id": "expand_draft",
            "buttonText": qsTr("Expand & Draft"),
            "icon": "icon-m-document",
            "instruction": "Please expand and draft the ideas in the active note into a well-structured AsciiDoc document with appropriate sections, headings, and detailed explanations. Call the edit_note tool with the complete expanded AsciiDoc content and filename."
        },
        {
            "id": "analyze_external",
            "buttonText": qsTr("External Text"),
            "icon": "icon-m-website",
            "instruction": "Please analyze the following external text or content, summarize key points, and extract relevant action items into structured AsciiDoc."
        }
    ]

    property var customAiInstructions: {
        var raw = customAiInstructionsConf.value
        if (raw && typeof raw === "string" && raw.trim().length > 0) {
            try {
                var parsed = JSON.parse(raw)
                if (Array.isArray(parsed) && parsed.length > 0) {
                    return parsed
                }
            } catch (e) {
                console.log("Error parsing custom AI instructions:", e)
            }
        }
        return defaultCustomAiInstructions
    }

    function isDefaultAiInstruction(id) {
        if (!id) return false
        for (var i = 0; i < defaultCustomAiInstructions.length; i++) {
            if (defaultCustomAiInstructions[i].id === id) {
                return true
            }
        }
        return false
    }

    function getDefaultAiInstruction(id) {
        if (!id) return null
        for (var i = 0; i < defaultCustomAiInstructions.length; i++) {
            if (defaultCustomAiInstructions[i].id === id) {
                return defaultCustomAiInstructions[i]
            }
        }
        return null
    }

    function saveCustomAiInstruction(item) {
        var list = []
        var current = customAiInstructions
        for (var i = 0; i < current.length; i++) {
            list.push(current[i])
        }
        var foundIndex = -1
        var targetId = item.id || ""
        if (targetId.length > 0) {
            for (var j = 0; j < list.length; j++) {
                if (list[j].id === targetId) {
                    foundIndex = j
                    break
                }
            }
        } else {
            targetId = "custom_" + Date.now()
            item.id = targetId
        }

        if (foundIndex >= 0) {
            list[foundIndex] = item
        } else {
            list.push(item)
        }
        customAiInstructionsConf.value = JSON.stringify(list)
    }

    function deleteCustomAiInstruction(id) {
        var current = customAiInstructions
        var filtered = []
        for (var i = 0; i < current.length; i++) {
            if (current[i].id !== id) {
                filtered.push(current[i])
            }
        }
        customAiInstructionsConf.value = JSON.stringify(filtered)
    }

    function resetSingleAiInstruction(id) {
        var def = getDefaultAiInstruction(id)
        if (!def) return
        saveCustomAiInstruction(def)
    }

    function resetCustomAiInstructions() {
        var current = customAiInstructions
        var defaultIds = {}
        for (var i = 0; i < defaultCustomAiInstructions.length; i++) {
            defaultIds[defaultCustomAiInstructions[i].id] = true
        }

        // Retain all custom user-created instructions (non-vendored)
        var userCustomList = []
        for (var j = 0; j < current.length; j++) {
            if (current[j] && current[j].id && !defaultIds[current[j].id]) {
                userCustomList.push(current[j])
            }
        }

        // Construct list: all original default vendored instructions + retained user custom instructions
        var resultList = []
        for (var k = 0; k < defaultCustomAiInstructions.length; k++) {
            resultList.push(defaultCustomAiInstructions[k])
        }
        for (var m = 0; m < userCustomList.length; m++) {
            resultList.push(userCustomList[m])
        }

        customAiInstructionsConf.value = JSON.stringify(resultList)
    }

    function setFontScale(scale) {
        fontSizeScaleConf.value = scale
    }

    function setFontFamily(family) {
        fontFamilyConf.value = family
    }

    function setCodeFontScale(scale) {
        codeFontScaleConf.value = scale
    }

    function setTocCollapseThreshold(threshold) {
        tocCollapseThresholdConf.value = threshold
    }

    function setPreviewScale(scale) {
        previewScaleConf.value = scale
    }

    function setGridColumns(cols) {
        gridColumnsConf.value = cols
    }

    function setDropComments(drop) {
        dropCommentsConf.value = drop
        bridge.set_drop_comments(drop)
    }

    function setAllowExternalImages(allow) {
        allowExternalImagesConf.value = allow
    }

    function setAutostartWebServer(val) {
        autostartWebServerConf.value = val
    }

    function setRejectPublicNetworks(val) {
        rejectPublicNetworksConf.value = val
        bridge.set_reject_public_networks(val)
    }

    function setBindAddress(addr) {
        bindAddressConf.value = addr
        bridge.set_bind_address(addr)
    }

    function setSessionExpiryHours(hours) {
        sessionExpiryHoursConf.value = hours
        if (typeof bridge !== "undefined" && bridge && typeof bridge.set_session_expiry_hours === "function") {
            bridge.set_session_expiry_hours(hours)
        }
    }

    function setJournalEnabled(val) {
        journalEnabledConf.value = val
    }

    function setGroupDisplayDepth(val) {
        groupDisplayDepthConf.value = val
        bridge.set_group_display_depth(val)
    }

    function setAiEnabled(val) {
        aiEnabledConf.value = val
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
        aiProviderConf.value = provider
        syncAiConfig()
    }

    function setAiEndpoint(endpoint) {
        aiEndpointConf.value = endpoint
        syncAiConfig()
    }

    function setAiModel(model) {
        aiModelConf.value = model
        syncAiConfig()
    }

    function setAiApiKey(key) {
        aiApiKeyConf.value = key
        syncAiConfig()
    }

    function setAiTimeout(secs) {
        aiTimeoutConf.value = secs
        syncAiConfig()
    }

    function setAiAutoAllowRead(val) {
        aiAutoAllowReadConf.value = val
        syncAiConfig()
    }

    function setAiAutoAllowCreate(val) {
        aiAutoAllowCreateConf.value = val
        syncAiConfig()
    }

    function setAiRequireConfirmEdit(val) {
        aiRequireConfirmEditConf.value = val
        syncAiConfig()
    }

    function setAiAllowFetchUrl(val) {
        aiAllowFetchUrlConf.value = val
        syncAiConfig()
    }

    function setAiAllowSelfSigned(val) {
        aiAllowSelfSignedConf.value = val
        syncAiConfig()
    }

    function setSttEnabled(val) {
        sttEnabledConf.value = val
    }

    function setSttModel(modelId) {
        sttModelConf.value = modelId
        if (typeof speechBridge !== "undefined" && speechBridge && typeof speechBridge.set_active_model === "function") {
            speechBridge.set_active_model(modelId)
        }
    }

    function openAssistant(contextFilename, contextContent, extraContext) {
        pageStack.push(Qt.resolvedUrl("pages/AssistantPage.qml"), {
            contextFilename: contextFilename || "",
            contextContent: contextContent || "",
            extraContext: extraContext || ""
        })
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

    Timer {
        id: startupTimer
        interval: 50
        running: false
        repeat: true
        onTriggered: {
            if (!bridge.initialized) {
                bridge.load_main_page_data()
            } else {
                startupTimer.stop()
            }
        }
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
        syncTheme()
        if (app.autostartWebServer) {
            bridge.start_web_server()
            syncTheme()
        }
        pageStack.forceActiveFocus()
        startupTimer.start()
    }

    Timer {
        id: pollTimer
        interval: 50
        running: bridge.is_loading
        repeat: true
        onTriggered: bridge.poll_results()
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
    }

    SpeechBridge {
        id: speechBridge

        onActive_model_changed: {
            if (active_model_id && active_model_id.length > 0 && active_model_id !== app.sttModel) {
                sttModelConf.value = active_model_id
            }
        }
    }

    Timer {
        id: speechPollTimer
        interval: 50
        running: typeof speechBridge !== "undefined" && speechBridge && (speechBridge.is_downloading || speechBridge.is_transcribing || speechBridge.is_recording)
        repeat: true
        onTriggered: {
            speechBridge.poll_worker()
        }
    }

    initialPage: Component { MainPage {} }
    cover: Component { CoverPage {} }

    Rectangle {
        id: notification
        property alias text: notificationLabel.text

        function show() {
            notification.opacity = 1.0
            hideTimer.restart()
        }

        anchors.top: parent.top
        anchors.topMargin: Theme.paddingLarge
        anchors.horizontalCenter: parent.horizontalCenter
        width: Math.min(parent.width - Theme.horizontalPageMargin * 2, notificationLabel.implicitWidth + Theme.paddingLarge * 2)
        height: Math.max(Theme.itemSizeExtraSmall, notificationLabel.implicitHeight + Theme.paddingSmall * 2)
        color: Qt.tint(
                   Theme.rgba(Theme.overlayBackgroundColor, Theme.opacityOverlay),
                   Theme.rgba(Theme.highlightBackgroundColor, Theme.highlightBackgroundOpacity))
        border.color: Theme.rgba(Theme.highlightColor, 0.4)
        border.width: 1
        radius: Theme.paddingSmall
        opacity: 0.0
        z: 1000

        Behavior on opacity { FadeAnimation {} }

        Label {
            id: notificationLabel
            anchors.fill: parent
            anchors.margins: Theme.paddingSmall
            horizontalAlignment: Text.AlignHCenter
            verticalAlignment: Text.AlignVCenter
            wrapMode: Text.Wrap
            color: Theme.primaryColor
            font.pixelSize: Theme.fontSizeExtraSmall
        }

        Timer {
            id: hideTimer
            interval: 3500
            onTriggered: notification.opacity = 0.0
        }

        MouseArea {
            anchors.fill: parent
            enabled: notification.opacity > 0
            onClicked: notification.opacity = 0.0
        }
    }
}
