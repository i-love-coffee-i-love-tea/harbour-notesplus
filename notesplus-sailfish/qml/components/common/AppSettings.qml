import QtQuick 2.6
import Nemo.Configuration 1.0
import "../../js/AiInstructionsManager.js" as AiInstructions

Item {
    id: settings

    readonly property string defaultAiEndpoint: "http://localhost:11434"

    ConfigurationValue {
        id: fontSizeScaleConf
        key: "/apps/harbour-notesplus/font_size_scale"
        defaultValue: 1.0
    }

    ConfigurationValue {
        id: fontFamilyConf
        key: "/apps/harbour-notesplus/font_family"
        defaultValue: ""
    }

    ConfigurationValue {
        id: codeFontScaleConf
        key: "/apps/harbour-notesplus/code_font_scale"
        defaultValue: 1.0
    }

    ConfigurationValue {
        id: tocCollapseThresholdConf
        key: "/apps/harbour-notesplus/toc_collapse_threshold"
        defaultValue: 5
    }

    ConfigurationValue {
        id: previewScaleConf
        key: "/apps/harbour-notesplus/preview_scale"
        defaultValue: 0.52
    }

    ConfigurationValue {
        id: gridColumnsConf
        key: "/apps/harbour-notesplus/grid_columns"
        defaultValue: 2
    }

    ConfigurationValue {
        id: dropCommentsConf
        key: "/apps/harbour-notesplus/drop_comments"
        defaultValue: true
    }

    ConfigurationValue {
        id: allowExternalImagesConf
        key: "/apps/harbour-notesplus/allow_external_images"
        defaultValue: true
    }

    ConfigurationValue {
        id: autostartWebServerConf
        key: "/apps/harbour-notesplus/autostart_web_server"
        defaultValue: false
    }

    ConfigurationValue {
        id: rejectPublicNetworksConf
        key: "/apps/harbour-notesplus/reject_public_networks"
        defaultValue: true
    }

    ConfigurationValue {
        id: bindAddressConf
        key: "/apps/harbour-notesplus/bind_address"
        defaultValue: "0.0.0.0"
    }

    ConfigurationValue {
        id: sessionExpiryHoursConf
        key: "/apps/harbour-notesplus/session_expiry_hours"
        defaultValue: 24
    }

    ConfigurationValue {
        id: groupDisplayDepthConf
        key: "/apps/harbour-notesplus/group_display_depth"
        defaultValue: 2
    }

    ConfigurationValue {
        id: journalEnabledConf
        key: "/apps/harbour-notesplus/journal_enabled"
        defaultValue: true
    }

    ConfigurationValue {
        id: aiEnabledConf
        key: "/apps/harbour-notesplus/ai_enabled"
        defaultValue: true
    }

    ConfigurationValue {
        id: aiProviderConf
        key: "/apps/harbour-notesplus/ai_provider"
        defaultValue: "ollama"
    }

    ConfigurationValue {
        id: aiEndpointConf
        key: "/apps/harbour-notesplus/ai_endpoint"
        defaultValue: defaultAiEndpoint
    }

    ConfigurationValue {
        id: aiModelConf
        key: "/apps/harbour-notesplus/ai_model"
        defaultValue: "llama3.2"
    }

    ConfigurationValue {
        id: aiApiKeyConf
        key: "/apps/harbour-notesplus/ai_api_key"
        defaultValue: ""
    }

    ConfigurationValue {
        id: aiTimeoutConf
        key: "/apps/harbour-notesplus/ai_timeout_secs"
        defaultValue: 90
    }

    ConfigurationValue {
        id: aiAutoAllowReadConf
        key: "/apps/harbour-notesplus/ai_auto_allow_read"
        defaultValue: true
    }

    ConfigurationValue {
        id: aiAutoAllowCreateConf
        key: "/apps/harbour-notesplus/ai_auto_allow_create"
        defaultValue: true
    }

    ConfigurationValue {
        id: aiRequireConfirmEditConf
        key: "/apps/harbour-notesplus/ai_require_confirm_edit"
        defaultValue: true
    }

    ConfigurationValue {
        id: aiAllowFetchUrlConf
        key: "/apps/harbour-notesplus/ai_allow_fetch_url"
        defaultValue: true
    }

    ConfigurationValue {
        id: aiAllowSelfSignedConf
        key: "/apps/harbour-notesplus/ai_allow_self_signed"
        defaultValue: false
    }

    ConfigurationValue {
        id: customAiInstructionsConf
        key: "/apps/harbour-notesplus/custom_ai_instructions"
        defaultValue: ""
    }

    ConfigurationValue {
        id: aiSystemPromptConf
        key: "/apps/harbour-notesplus/ai_system_prompt"
        defaultValue: ""
    }

    ConfigurationValue {
        id: sttEnabledConf
        key: "/apps/harbour-notesplus/stt_enabled"
        defaultValue: true
    }

    ConfigurationValue {
        id: sttModelConf
        key: "/apps/harbour-notesplus/stt_model"
        defaultValue: ""
    }

    ConfigurationValue {
        id: notesPathConf
        key: "/apps/harbour-notesplus/notes_path"
        defaultValue: ""
    }

    // Exposed configuration properties
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

    property bool aiEnabled: aiEnabledConf.value !== undefined ? aiEnabledConf.value : true
    property string aiProvider: aiProviderConf.value !== undefined ? aiProviderConf.value : "ollama"
    property string aiEndpoint: aiEndpointConf.value !== undefined ? aiEndpointConf.value : defaultAiEndpoint
    property string aiModel: aiModelConf.value !== undefined ? aiModelConf.value : "llama3.2"
    property string aiApiKey: aiApiKeyConf.value !== undefined ? aiApiKeyConf.value : ""
    property int aiTimeout: aiTimeoutConf.value !== undefined ? aiTimeoutConf.value : 90
    property bool aiAutoAllowRead: aiAutoAllowReadConf.value !== undefined ? aiAutoAllowReadConf.value : true
    property bool aiAutoAllowCreate: aiAutoAllowCreateConf.value !== undefined ? aiAutoAllowCreateConf.value : true
    property bool aiRequireConfirmEdit: aiRequireConfirmEditConf.value !== undefined ? aiRequireConfirmEditConf.value : true
    property bool aiAllowFetchUrl: aiAllowFetchUrlConf.value !== undefined ? aiAllowFetchUrlConf.value : true
    property bool aiAllowSelfSigned: aiAllowSelfSignedConf.value !== undefined ? aiAllowSelfSignedConf.value : false
    property string customAiInstructionsRaw: customAiInstructionsConf.value !== undefined ? customAiInstructionsConf.value : ""
    property string aiSystemPrompt: aiSystemPromptConf.value !== undefined ? aiSystemPromptConf.value : ""

    readonly property var defaultCustomAiInstructions: AiInstructions.defaultCustomAiInstructions
    property var customAiInstructions: AiInstructions.parseCustomAiInstructions(customAiInstructionsRaw)

    property bool sttEnabled: sttEnabledConf.value !== undefined ? sttEnabledConf.value : true
    property string sttModel: sttModelConf.value !== undefined ? sttModelConf.value : ""
    property string notesPath: notesPathConf.value !== undefined ? notesPathConf.value : ""

    // Setters
    function setFontScale(v) { fontSizeScaleConf.value = v; }
    function setFontFamily(v) { fontFamilyConf.value = v; }
    function setCodeFontScale(v) { codeFontScaleConf.value = v; }
    function setTocCollapseThreshold(v) { tocCollapseThresholdConf.value = v; }
    function setPreviewScale(v) { previewScaleConf.value = v; }
    function setGridColumns(v) { gridColumnsConf.value = v; }
    function setDropComments(v) { dropCommentsConf.value = v; }
    function setAllowExternalImages(v) { allowExternalImagesConf.value = v; }
    function setAutostartWebServer(v) { autostartWebServerConf.value = v; }
    function setRejectPublicNetworks(v) { rejectPublicNetworksConf.value = v; }
    function setBindAddress(v) { bindAddressConf.value = v; }
    function setSessionExpiryHours(v) { sessionExpiryHoursConf.value = v; }
    function setGroupDisplayDepth(v) { groupDisplayDepthConf.value = v; }
    function setJournalEnabled(v) { journalEnabledConf.value = v; }

    function setAiEnabled(v) { aiEnabledConf.value = v; }
    function setAiProvider(v) { aiProviderConf.value = v; }
    function setAiEndpoint(v) { aiEndpointConf.value = v; }
    function setAiModel(v) { aiModelConf.value = v; }
    function setAiApiKey(v) { aiApiKeyConf.value = v; }
    function setAiTimeout(v) { aiTimeoutConf.value = v; }
    function setAiAutoAllowRead(v) { aiAutoAllowReadConf.value = v; }
    function setAiAutoAllowCreate(v) { aiAutoAllowCreateConf.value = v; }
    function setAiRequireConfirmEdit(v) { aiRequireConfirmEditConf.value = v; }
    function setAiAllowFetchUrl(v) { aiAllowFetchUrlConf.value = v; }
    function setAiAllowSelfSigned(v) { aiAllowSelfSignedConf.value = v; }
    function setCustomAiInstructionsRaw(v) { customAiInstructionsConf.value = v; }
    function setAiSystemPrompt(v) { aiSystemPromptConf.value = v; }

    function isDefaultAiInstruction(id) { return AiInstructions.isDefaultAiInstruction(id) }
    function getDefaultAiInstruction(id) { return AiInstructions.getDefaultAiInstruction(id) }

    function saveCustomAiInstruction(item) {
        customAiInstructionsConf.value = AiInstructions.saveCustomAiInstruction(customAiInstructions, item)
        customAiInstructions = AiInstructions.parseCustomAiInstructions(customAiInstructionsConf.value)
    }

    function deleteCustomAiInstruction(id) {
        customAiInstructionsConf.value = AiInstructions.deleteCustomAiInstruction(customAiInstructions, id)
        customAiInstructions = AiInstructions.parseCustomAiInstructions(customAiInstructionsConf.value)
    }

    function resetSingleAiInstruction(id) {
        var def = AiInstructions.getDefaultAiInstruction(id)
        if (!def) return
        saveCustomAiInstruction(def)
    }

    function resetCustomAiInstructions() {
        customAiInstructionsConf.value = AiInstructions.resetCustomAiInstructions(customAiInstructions)
        customAiInstructions = AiInstructions.parseCustomAiInstructions(customAiInstructionsConf.value)
    }

    function setSttEnabled(v) { sttEnabledConf.value = v; }
    function setSttModel(v) { sttModelConf.value = v; }
    function setNotesPath(v) { notesPathConf.value = v; }
}
