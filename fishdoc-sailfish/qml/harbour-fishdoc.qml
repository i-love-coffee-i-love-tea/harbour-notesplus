import QtQuick 2.6
import Sailfish.Silica 1.0
import Nemo.Configuration 1.0
import harbour.fishdoc 1.0
import "pages"
import "cover"

ApplicationWindow {
    id: app
    _defaultPageOrientations: Orientation.All

    ConfigurationValue {
        id: fontSizeScaleConf
        key: "/apps/harbour-fishdoc/font_size_scale"
        defaultValue: 1.0
    }

    ConfigurationValue {
        id: fontFamilyConf
        key: "/apps/harbour-fishdoc/font_family"
        defaultValue: ""
    }

    ConfigurationValue {
        id: codeFontScaleConf
        key: "/apps/harbour-fishdoc/code_font_scale"
        defaultValue: 1.0
    }

    ConfigurationValue {
        id: tocCollapseThresholdConf
        key: "/apps/harbour-fishdoc/toc_collapse_threshold"
        defaultValue: 5
    }

    ConfigurationValue {
        id: previewScaleConf
        key: "/apps/harbour-fishdoc/preview_scale"
        defaultValue: 0.52
    }

    ConfigurationValue {
        id: dropCommentsConf
        key: "/apps/harbour-fishdoc/drop_comments"
        defaultValue: true
    }

    ConfigurationValue {
        id: allowExternalImagesConf
        key: "/apps/harbour-fishdoc/allow_external_images"
        defaultValue: true
    }

    ConfigurationValue {
        id: autostartWebServerConf
        key: "/apps/harbour-fishdoc/autostart_web_server"
        defaultValue: false
    }

    ConfigurationValue {
        id: journalEnabledConf
        key: "/apps/harbour-fishdoc/journal_enabled"
        defaultValue: true
    }

    ConfigurationValue {
        id: aiEnabledConf
        key: "/apps/harbour-fishdoc/ai_enabled"
        defaultValue: true
    }

    ConfigurationValue {
        id: aiProviderConf
        key: "/apps/harbour-fishdoc/ai_provider"
        defaultValue: "ollama"
    }

    ConfigurationValue {
        id: aiEndpointConf
        key: "/apps/harbour-fishdoc/ai_endpoint"
        defaultValue: "http://192.168.1.1:11434"
    }

    ConfigurationValue {
        id: aiModelConf
        key: "/apps/harbour-fishdoc/ai_model"
        defaultValue: "llama3.2"
    }

    ConfigurationValue {
        id: aiApiKeyConf
        key: "/apps/harbour-fishdoc/ai_api_key"
        defaultValue: ""
    }

    ConfigurationValue {
        id: aiTimeoutConf
        key: "/apps/harbour-fishdoc/ai_timeout_secs"
        defaultValue: 90
    }

    ConfigurationValue {
        id: aiAutoAllowReadConf
        key: "/apps/harbour-fishdoc/ai_auto_allow_read"
        defaultValue: true
    }

    ConfigurationValue {
        id: aiAutoAllowCreateConf
        key: "/apps/harbour-fishdoc/ai_auto_allow_create"
        defaultValue: true
    }

    ConfigurationValue {
        id: aiRequireConfirmEditConf
        key: "/apps/harbour-fishdoc/ai_require_confirm_edit"
        defaultValue: true
    }

    property real fontScale: fontSizeScaleConf.value !== undefined && fontSizeScaleConf.value > 0 ? fontSizeScaleConf.value : 1.0
    property string docFontFamily: fontFamilyConf.value !== undefined ? fontFamilyConf.value : ""
    property real codeFontScale: codeFontScaleConf.value !== undefined && codeFontScaleConf.value > 0 ? codeFontScaleConf.value : 1.0
    property int tocCollapseThreshold: tocCollapseThresholdConf.value !== undefined ? tocCollapseThresholdConf.value : 5
    property real previewScale: previewScaleConf.value !== undefined && previewScaleConf.value > 0 ? previewScaleConf.value : 0.52
    property bool dropComments: dropCommentsConf.value !== undefined ? dropCommentsConf.value : true
    property bool allowExternalImages: allowExternalImagesConf.value !== undefined ? allowExternalImagesConf.value : true
    property bool autostartWebServer: autostartWebServerConf.value !== undefined ? autostartWebServerConf.value : false
    property bool journalEnabled: journalEnabledConf.value !== undefined ? journalEnabledConf.value : true
    property bool aiEnabled: aiEnabledConf.value !== undefined ? aiEnabledConf.value : true

    property string aiProvider: aiProviderConf.value !== undefined ? aiProviderConf.value : "ollama"
    property string aiEndpoint: aiEndpointConf.value !== undefined ? aiEndpointConf.value : "http://192.168.1.1:11434"
    property string aiModel: aiModelConf.value !== undefined ? aiModelConf.value : "llama3.2"
    property string aiApiKey: aiApiKeyConf.value !== undefined ? aiApiKeyConf.value : ""
    property int aiTimeout: aiTimeoutConf.value !== undefined ? aiTimeoutConf.value : 90
    property bool aiAutoAllowRead: aiAutoAllowReadConf.value !== undefined ? aiAutoAllowReadConf.value : true
    property bool aiAutoAllowCreate: aiAutoAllowCreateConf.value !== undefined ? aiAutoAllowCreateConf.value : true
    property bool aiRequireConfirmEdit: aiRequireConfirmEditConf.value !== undefined ? aiRequireConfirmEditConf.value : true

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

    function setJournalEnabled(val) {
        journalEnabledConf.value = val
    }

    function setAiEnabled(val) {
        aiEnabledConf.value = val
    }

    function setAiProvider(provider) {
        aiProviderConf.value = provider
    }

    function setAiEndpoint(endpoint) {
        aiEndpointConf.value = endpoint
    }

    function setAiModel(model) {
        aiModelConf.value = model
    }

    function setAiApiKey(key) {
        aiApiKeyConf.value = key
    }

    function setAiTimeout(secs) {
        aiTimeoutConf.value = secs
    }

    function setAiAutoAllowRead(val) {
        aiAutoAllowReadConf.value = val
    }

    function setAiAutoAllowCreate(val) {
        aiAutoAllowCreateConf.value = val
    }

    function setAiRequireConfirmEdit(val) {
        aiRequireConfirmEditConf.value = val
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
        interval: 10
        running: false
        repeat: false
        onTriggered: bridge.load_main_page_data()
    }

    Component.onCompleted: {
        bridge.set_drop_comments(app.dropComments)
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
