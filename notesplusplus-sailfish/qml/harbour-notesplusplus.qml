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
        id: aiAllowSelfSignedConf
        key: "/apps/harbour-notesplusplus/ai_allow_self_signed"
        defaultValue: false
    }

    ConfigurationValue {
        id: webAuthEnabledConf
        key: "/apps/harbour-notesplusplus/web_auth_enabled"
        defaultValue: false
    }

    ConfigurationValue {
        id: webAuthBasicEnabledConf
        key: "/apps/harbour-notesplusplus/web_auth_basic_enabled"
        defaultValue: true
    }

    ConfigurationValue {
        id: webAuthUsernameConf
        key: "/apps/harbour-notesplusplus/web_auth_username"
        defaultValue: "admin"
    }

    ConfigurationValue {
        id: webAuthPasswordConf
        key: "/apps/harbour-notesplusplus/web_auth_password"
        defaultValue: ""
    }

    ConfigurationValue {
        id: webAuthOauthEnabledConf
        key: "/apps/harbour-notesplusplus/web_auth_oauth_enabled"
        defaultValue: false
    }

    ConfigurationValue {
        id: webAuthOauthProviderNameConf
        key: "/apps/harbour-notesplusplus/web_auth_oauth_provider_name"
        defaultValue: "Authentik"
    }

    ConfigurationValue {
        id: webAuthOauthIssuerUrlConf
        key: "/apps/harbour-notesplusplus/web_auth_oauth_issuer_url"
        defaultValue: ""
    }

    ConfigurationValue {
        id: webAuthOauthClientIdConf
        key: "/apps/harbour-notesplusplus/web_auth_oauth_client_id"
        defaultValue: ""
    }

    ConfigurationValue {
        id: webAuthOauthClientSecretConf
        key: "/apps/harbour-notesplusplus/web_auth_oauth_client_secret"
        defaultValue: ""
    }

    ConfigurationValue {
        id: webAuthOauthAllowedEmailsConf
        key: "/apps/harbour-notesplusplus/web_auth_oauth_allowed_emails"
        defaultValue: ""
    }

    ConfigurationValue {
        id: webAuthOauthAllowSelfSignedConf
        key: "/apps/harbour-notesplusplus/web_auth_oauth_allow_self_signed"
        defaultValue: false
    }

    property real fontScale: fontSizeScaleConf.value !== undefined && fontSizeScaleConf.value > 0 ? fontSizeScaleConf.value : 1.0
    property string docFontFamily: fontFamilyConf.value !== undefined ? fontFamilyConf.value : ""
    property real codeFontScale: codeFontScaleConf.value !== undefined && codeFontScaleConf.value > 0 ? codeFontScaleConf.value : 1.0
    property int tocCollapseThreshold: tocCollapseThresholdConf.value !== undefined ? tocCollapseThresholdConf.value : 5
    property real previewScale: previewScaleConf.value !== undefined && previewScaleConf.value > 0 ? previewScaleConf.value : 0.52
    property bool dropComments: dropCommentsConf.value !== undefined ? dropCommentsConf.value : true
    property bool allowExternalImages: allowExternalImagesConf.value !== undefined ? allowExternalImagesConf.value : true
    property bool autostartWebServer: autostartWebServerConf.value !== undefined ? autostartWebServerConf.value : false
    property bool rejectPublicNetworks: rejectPublicNetworksConf.value !== undefined ? rejectPublicNetworksConf.value : true
    property bool journalEnabled: journalEnabledConf.value !== undefined ? journalEnabledConf.value : true
    property bool aiEnabled: aiEnabledConf.value !== undefined ? aiEnabledConf.value : true

    property string aiProvider: aiProviderConf.value !== undefined ? aiProviderConf.value : "ollama"
    property string aiEndpoint: aiEndpointConf.value !== undefined ? aiEndpointConf.value : app.defaultAiEndpoint
    property string aiModel: aiModelConf.value !== undefined ? aiModelConf.value : "llama3.2"
    property string aiApiKey: aiApiKeyConf.value !== undefined ? aiApiKeyConf.value : ""
    property int aiTimeout: aiTimeoutConf.value !== undefined ? aiTimeoutConf.value : 90
    property bool aiAutoAllowRead: aiAutoAllowReadConf.value !== undefined ? aiAutoAllowReadConf.value : true
    property bool aiAutoAllowCreate: aiAutoAllowCreateConf.value !== undefined ? aiAutoAllowCreateConf.value : true
    property bool aiRequireConfirmEdit: aiRequireConfirmEditConf.value !== undefined ? aiRequireConfirmEditConf.value : true
    property bool aiAllowSelfSigned: aiAllowSelfSignedConf.value !== undefined ? aiAllowSelfSignedConf.value : false

    property bool webAuthEnabled: webAuthEnabledConf.value !== undefined ? webAuthEnabledConf.value : false
    property bool webAuthBasicEnabled: webAuthBasicEnabledConf.value !== undefined ? webAuthBasicEnabledConf.value : true
    property string webAuthUsername: webAuthUsernameConf.value !== undefined ? webAuthUsernameConf.value : "admin"
    property string webAuthPassword: webAuthPasswordConf.value !== undefined ? webAuthPasswordConf.value : ""
    property bool webAuthOauthEnabled: webAuthOauthEnabledConf.value !== undefined ? webAuthOauthEnabledConf.value : false
    property string webAuthOauthProviderName: webAuthOauthProviderNameConf.value !== undefined ? webAuthOauthProviderNameConf.value : "Authentik"
    property string webAuthOauthIssuerUrl: webAuthOauthIssuerUrlConf.value !== undefined ? webAuthOauthIssuerUrlConf.value : ""
    property string webAuthOauthClientId: webAuthOauthClientIdConf.value !== undefined ? webAuthOauthClientIdConf.value : ""
    property string webAuthOauthClientSecret: webAuthOauthClientSecretConf.value !== undefined ? webAuthOauthClientSecretConf.value : ""
    property string webAuthOauthAllowedEmails: webAuthOauthAllowedEmailsConf.value !== undefined ? webAuthOauthAllowedEmailsConf.value : ""
    property bool webAuthOauthAllowSelfSigned: webAuthOauthAllowSelfSignedConf.value !== undefined ? webAuthOauthAllowSelfSignedConf.value : false

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

    function setJournalEnabled(val) {
        journalEnabledConf.value = val
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
                app.aiAllowSelfSigned !== undefined ? app.aiAllowSelfSigned : false
            )
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

    function setAiAllowSelfSigned(val) {
        aiAllowSelfSignedConf.value = val
        syncAiConfig()
    }

    function syncWebAuthConfig() {
        if (typeof bridge !== "undefined" && bridge && typeof bridge.configure_auth === "function") {
            bridge.configure_auth(
                app.webAuthEnabled || false,
                app.webAuthBasicEnabled !== undefined ? app.webAuthBasicEnabled : true,
                app.webAuthUsername || "admin",
                app.webAuthPassword || "",
                app.webAuthOauthEnabled || false,
                app.webAuthOauthProviderName || "Authentik",
                app.webAuthOauthIssuerUrl || "",
                app.webAuthOauthClientId || "",
                app.webAuthOauthClientSecret || "",
                app.webAuthOauthAllowedEmails || "",
                app.webAuthOauthAllowSelfSigned || false
            )
        }
    }

    function setWebAuthEnabled(val) {
        webAuthEnabledConf.value = val
        syncWebAuthConfig()
    }

    function setWebAuthBasicEnabled(val) {
        webAuthBasicEnabledConf.value = val
        syncWebAuthConfig()
    }

    function setWebAuthUsername(user) {
        webAuthUsernameConf.value = user
        syncWebAuthConfig()
    }

    function setWebAuthPassword(pass) {
        webAuthPasswordConf.value = pass
        syncWebAuthConfig()
    }

    function setWebAuthOauthEnabled(val) {
        webAuthOauthEnabledConf.value = val
        syncWebAuthConfig()
    }

    function setWebAuthOauthProviderName(name) {
        webAuthOauthProviderNameConf.value = name
        syncWebAuthConfig()
    }

    function setWebAuthOauthIssuerUrl(url) {
        webAuthOauthIssuerUrlConf.value = url
        syncWebAuthConfig()
    }

    function setWebAuthOauthClientId(id) {
        webAuthOauthClientIdConf.value = id
        syncWebAuthConfig()
    }

    function setWebAuthOauthClientSecret(secret) {
        webAuthOauthClientSecretConf.value = secret
        syncWebAuthConfig()
    }

    function setWebAuthOauthAllowedEmails(emails) {
        webAuthOauthAllowedEmailsConf.value = emails
        syncWebAuthConfig()
    }

    function setWebAuthOauthAllowSelfSigned(val) {
        webAuthOauthAllowSelfSignedConf.value = val
        syncWebAuthConfig()
    }

    function openAssistant(contextFilename, contextContent) {
        pageStack.push(Qt.resolvedUrl("pages/AssistantPage.qml"), {
            contextFilename: contextFilename || "",
            contextContent: contextContent || ""
        })
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
        syncWebAuthConfig()
        bridge.set_drop_comments(app.dropComments)
        bridge.set_reject_public_networks(app.rejectPublicNetworks)
        if (app.autostartWebServer) {
            bridge.start_web_server()
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

    Connections {
        target: RootApp
        onLastWindowClosed: Qt.quit()
    }

    NotesBridge {
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
        height: Theme.itemSizeExtraSmall
        color: Theme.highlightBackgroundColor
        opacity: 0.0
        z: 100

        Behavior on opacity { FadeAnimator {} }

        Label {
            id: notificationLabel
            anchors.centerIn: parent
            color: Theme.primaryColor
            font.pixelSize: Theme.fontSizeExtraSmall
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
