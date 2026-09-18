use std::fs;
use std::sync::Arc;
use notesplusplus_core::server::*;
use notesplusplus_core::agent::client::LlmConfig;
use notesplusplus_core::agent::permissions::PermissionConfig;
use serde_json::json;
use tempfile::tempdir;

#[test]
fn test_note_filename_sanitization() {
    assert_eq!(notesplusplus_core::page::sanitize_note_filename("My Note!"), "My_Note.adoc");
    assert_eq!(notesplusplus_core::page::sanitize_note_filename("  "), "Untitled.adoc");
    assert_eq!(notesplusplus_core::page::sanitize_note_filename("Hello World 123"), "Hello_World_123.adoc");
}

#[test]
fn test_list_all_notes_json() {
    let tmp = tempdir().unwrap();
    let notes_dir = tmp.path().join("notes");
    fs::create_dir_all(&notes_dir).unwrap();

    fs::write(notes_dir.join("alpha.adoc"), "= Alpha Note\nFirst line of content.").unwrap();
    fs::write(notes_dir.join("beta.adoc"), "= Beta Note\nSecond line of content.").unwrap();

    let json_str = list_all_notes_json(&notes_dir, None);
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    assert_eq!(parsed.as_array().unwrap().len(), 2);
}

#[test]
fn test_server_lifecycle_and_endpoints() {
    let tmp = tempdir().unwrap();
    let notes_dir = tmp.path().join("notes");
    let db_path = tmp.path().join("test.db");
    let backup_dir = tmp.path().join("backups");
    fs::create_dir_all(&notes_dir).unwrap();

    fs::write(notes_dir.join("welcome.adoc"), "= Welcome\nTest content for server.").unwrap();

    let server_handle = start_server_full(
        notes_dir.clone(),
        db_path,
        backup_dir,
        18920,
        None,
        None,
    ).expect("Server should start");

    assert!(server_handle.is_running());
    let port = server_handle.port();
    assert!(port >= 18920);

    // Test GET /
    let res_root = ureq::get(&format!("http://127.0.0.1:{}/", port)).call().unwrap();
    assert_eq!(res_root.status(), 200);
    let root_body = res_root.into_string().unwrap();
    assert!(root_body.contains("Notes Plus"));

    // Test GET /icon.png
    let res_icon = ureq::get(&format!("http://127.0.0.1:{}/icon.png", port)).call().unwrap();
    assert_eq!(res_icon.status(), 200);
    assert_eq!(res_icon.header("Content-Type").unwrap(), "image/png");

    // Test GET /app.js
    let res_js = ureq::get(&format!("http://127.0.0.1:{}/app.js", port)).call().unwrap();
    assert_eq!(res_js.status(), 200);
    let js_body = res_js.into_string().unwrap();
    assert!(js_body.contains("createApp"));

    // Test GET /vue.esm-browser.prod.js (vendored Vue)
    let res_vue = ureq::get(&format!("http://127.0.0.1:{}/vue.esm-browser.prod.js", port)).call().unwrap();
    assert_eq!(res_vue.status(), 200);
    let vue_body = res_vue.into_string().unwrap();
    assert!(vue_body.contains("vue"));

    // Test GET /vue.js alias
    let res_vue_alias = ureq::get(&format!("http://127.0.0.1:{}/vue.js", port)).call().unwrap();
    assert_eq!(res_vue_alias.status(), 200);

    // Test GET /style.css
    let res_css = ureq::get(&format!("http://127.0.0.1:{}/style.css", port)).call().unwrap();
    assert_eq!(res_css.status(), 200);

    // Authenticate session for API testing
    let sess = server_handle.context().session_store.create_session("admin", "code", 3600).unwrap();
    let session_cookie = format!("{}={}", notesplusplus_core::constants::SESSION_COOKIE_NAME, sess.id);

    // Test GET /api/notes
    let res_notes = ureq::get(&format!("http://127.0.0.1:{}/api/notes", port))
        .set("Cookie", &session_cookie)
        .call()
        .unwrap();
    assert_eq!(res_notes.status(), 200);
    let notes_json: serde_json::Value = res_notes.into_json().unwrap();
    assert!(!notes_json.as_array().unwrap().is_empty());
    assert!(notes_json[0].get("color").is_some());
    assert!(notes_json[0].get("id").is_some());

    // Test GET /api/tree
    let res_tree = ureq::get(&format!("http://127.0.0.1:{}/api/tree", port))
        .set("Cookie", &session_cookie)
        .call()
        .unwrap();
    assert_eq!(res_tree.status(), 200);
    let tree_json: serde_json::Value = res_tree.into_json().unwrap();
    assert!(tree_json.is_array());
    let tree_arr = tree_json.as_array().unwrap();
    assert!(!tree_arr.is_empty());
    let first_group = &tree_arr[0];
    let pages = first_group["pages"].as_array().unwrap();
    assert!(!pages.is_empty());
    assert!(pages[0].get("color").is_some());
    assert!(pages[0].get("preview_blocks").is_some());
    assert!(pages[0].get("preview_blocks_json").is_some());

    // Test GET /api/notes/welcome.adoc
    let res_note = ureq::get(&format!("http://127.0.0.1:{}/api/notes/welcome.adoc", port))
        .set("Cookie", &session_cookie)
        .call()
        .unwrap();
    assert_eq!(res_note.status(), 200);
    assert!(res_note.into_string().unwrap().contains("Test content"));

    // Test PUT /api/notes/welcome.adoc
    let put_res = ureq::put(&format!("http://127.0.0.1:{}/api/notes/welcome.adoc", port))
        .set("Cookie", &session_cookie)
        .set("Content-Type", "text/plain")
        .send_string("= Welcome\nUpdated content from PUT test.")
        .unwrap();
    assert_eq!(put_res.status(), 200);
    let updated_file = fs::read_to_string(notes_dir.join("welcome.adoc")).unwrap();
    assert!(updated_file.contains("Updated content from PUT test"));

    // Test POST /api/notes (create new)
    let create_res = ureq::post(&format!("http://127.0.0.1:{}/api/notes", port))
        .set("Cookie", &session_cookie)
        .send_json(json!({
            "title": "New Doc",
            "content": "= New Doc\nCreated via API"
        }))
        .unwrap();
    assert_eq!(create_res.status(), 200);
    assert!(notes_dir.join("New_Doc.adoc").exists());

    // Test POST /api/render
    let render_res = ureq::post(&format!("http://127.0.0.1:{}/api/render", port))
        .set("Cookie", &session_cookie)
        .send_json(json!({
            "content": "= Header\n* Bullet item\n"
        }))
        .unwrap();
    assert_eq!(render_res.status(), 200);
    let render_html = render_res.into_string().unwrap();
    assert!(render_html.contains("Header") && render_html.contains("Bullet item"));

    // Test POST /api/blocks/parse
    let parse_res = ureq::post(&format!("http://127.0.0.1:{}/api/blocks/parse", port))
        .set("Cookie", &session_cookie)
        .send_json(json!({
            "content": "= Heading 1\n\nParagraph text\n\n* [ ] Task 1"
        }))
        .unwrap();
    assert_eq!(parse_res.status(), 200);
    let parse_json: serde_json::Value = parse_res.into_json().unwrap();
    assert!(parse_json.get("blocks").and_then(|b| b.as_array()).unwrap().len() >= 3);

    // Test POST /api/notes/welcome.adoc/toggle (checklist toggle)
    fs::write(notes_dir.join("welcome.adoc"), "= Tasks\n* [ ] Task 1\n* [x] Task 2").unwrap();
    let toggle_res = ureq::post(&format!("http://127.0.0.1:{}/api/notes/welcome.adoc/toggle", port))
        .set("Cookie", &session_cookie)
        .send_json(json!({
            "item_index": 0,
            "checked": true
        }))
        .unwrap();
    assert_eq!(toggle_res.status(), 200);
    let toggled_file = fs::read_to_string(notes_dir.join("welcome.adoc")).unwrap();
    assert!(toggled_file.contains("* [x] Task 1"));

    // Test POST /api/ai/config (update config)
    let ai_update_res = ureq::post(&format!("http://127.0.0.1:{}/api/ai/config", port))
        .set("Cookie", &session_cookie)
        .send_json(json!({
            "provider": "openai",
            "model": "mistral",
            "system_prompt": "You are a concise technical writer."
        }))
        .unwrap();
    assert_eq!(ai_update_res.status(), 200);

    // Verify updated config
    let ai_cfg_res2 = ureq::get(&format!("http://127.0.0.1:{}/api/ai/config", port))
        .set("Cookie", &session_cookie)
        .call()
        .unwrap();
    let ai_cfg_json2: serde_json::Value = ai_cfg_res2.into_json().unwrap();
    assert_eq!(ai_cfg_json2.get("provider").unwrap(), "openai");
    assert_eq!(ai_cfg_json2.get("model").unwrap(), "mistral");
    assert_eq!(ai_cfg_json2.get("system_prompt").unwrap(), "You are a concise technical writer.");
    // Verify endpoint, tokens, keys, or cert options are never exposed in responses to web UI
    assert!(ai_cfg_json2.get("endpoint").is_none());
    assert!(ai_cfg_json2.get("api_key").is_none());
    assert!(ai_cfg_json2.get("has_key").is_none());
    assert!(ai_cfg_json2.get("allow_self_signed").is_none());

    // Test GET /raw/welcome.adoc
    let raw_res = ureq::get(&format!("http://127.0.0.1:{}/raw/welcome.adoc", port))
        .set("Cookie", &session_cookie)
        .call()
        .unwrap();
    assert_eq!(raw_res.status(), 200);
    assert!(raw_res.into_string().unwrap().contains("Task"));

    // Test GET /export/welcome.adoc
    let export_res = ureq::get(&format!("http://127.0.0.1:{}/export/welcome.adoc", port))
        .set("Cookie", &session_cookie)
        .call()
        .unwrap();
    assert_eq!(export_res.status(), 200);
    assert!(export_res.into_string().unwrap().contains("html"));

    // Test DELETE /api/notes/New_Doc.adoc
    let del_res = ureq::delete(&format!("http://127.0.0.1:{}/api/notes/New_Doc.adoc", port))
        .set("Cookie", &session_cookie)
        .call()
        .unwrap();
    assert_eq!(del_res.status(), 200);
    assert!(!notes_dir.join("New_Doc.adoc").exists());

    // Stop server
    server_handle.stop();
}

#[test]
fn test_server_phone_auth_challenge_flow_and_multi_request_superseding() {
    let tmp = tempdir().unwrap();
    let notes_dir = tmp.path().join("notes");
    let db_path = tmp.path().join("test_phone_auth.db");
    let backup_dir = tmp.path().join("backups");
    fs::create_dir_all(&notes_dir).unwrap();

    fs::write(notes_dir.join("protected.adoc"), "= Protected\nContent only for authenticated users.").unwrap();

    let auth_cfg = auth::AuthConfig::default();

    let config = ServerConfig {
        notes_dir: notes_dir.clone(),
        assets_dir: notes_dir.parent().unwrap_or(&notes_dir).join("assets"),
        db_path,
        backup_dir,
        port: 18945,
        bind_address: "127.0.0.1".to_string(),
        llm_config: LlmConfig::default(),
        permission_config: PermissionConfig::default(),
        auth_config: auth_cfg,
        enable_tls: false,
        tls_cert_path: None,
        tls_key_path: None,
        reject_public_networks: true,
    };

    let server_handle = start_server_with_config(config).expect("Auth server should start");
    let port = server_handle.port();

    // 1. Initial unauthenticated access fails with 401
    let unauth_res = ureq::get(&format!("http://127.0.0.1:{}/api/notes", port)).call();
    assert!(unauth_res.is_err());

    // 2. Client 1 initiates phone authorization challenge
    let init_res1 = ureq::post(&format!("http://127.0.0.1:{}/api/auth/code/initiate", port))
        .send_json(json!({}))
        .unwrap();
    assert_eq!(init_res1.status(), 200);
    let init_json1: serde_json::Value = init_res1.into_json().unwrap();
    assert_eq!(init_json1["ok"], true);
    let c1_id = init_json1["challenge_id"].as_str().unwrap().to_string();
    let c1_code = init_json1["verification_code"].as_str().unwrap().to_string();
    assert!(!c1_id.is_empty());
    assert_eq!(c1_code.len(), 4);

    // Verify pending challenge is set on the server context
    assert_eq!(
        server_handle.context().pending_auth_challenge.lock().unwrap().as_deref(),
        Some(c1_id.as_str())
    );

    // 3. Client 1 polls status -> status: "pending"
    let poll1 = ureq::get(&format!("http://127.0.0.1:{}/api/auth/code/status?challenge_id={}", port, c1_id))
        .call()
        .unwrap();
    let poll_json1: serde_json::Value = poll1.into_json().unwrap();
    assert_eq!(poll_json1["status"], "pending");

    // 4. Client 2 (or a repeated burst of requests) initiates another challenge
    let init_res2 = ureq::post(&format!("http://127.0.0.1:{}/api/auth/code/initiate", port))
        .send_json(json!({}))
        .unwrap();
    assert_eq!(init_res2.status(), 200);
    let init_json2: serde_json::Value = init_res2.into_json().unwrap();
    assert_eq!(init_json2["ok"], true);
    let c2_id = init_json2["challenge_id"].as_str().unwrap().to_string();
    let c2_code = init_json2["verification_code"].as_str().unwrap().to_string();
    assert_ne!(c1_id, c2_id);
    assert_eq!(c2_code.len(), 4);

    // Server context's pending challenge should now point to Challenge 2 (superseding Challenge 1)
    assert_eq!(
        server_handle.context().pending_auth_challenge.lock().unwrap().as_deref(),
        Some(c2_id.as_str())
    );

    // 5. Phone accepts the currently displayed Challenge 2
    server_handle.context().auth_challenges.approve_challenge(&c2_id);
    server_handle.context().clear_auth_challenge();

    // 6. Client 2 polls status -> approved, gets session cookie
    let poll2 = ureq::get(&format!("http://127.0.0.1:{}/api/auth/code/status?challenge_id={}", port, c2_id))
        .call()
        .unwrap();
    let session_cookie = poll2
        .header("Set-Cookie")
        .expect("Should set session cookie")
        .split(';')
        .next()
        .unwrap()
        .to_string();
    let poll_json2: serde_json::Value = poll2.into_json().unwrap();
    assert_eq!(poll_json2["status"], "approved");

    // 7. Client 2 uses cookie to access protected notes API
    let notes_res = ureq::get(&format!("http://127.0.0.1:{}/api/notes", port))
        .set("Cookie", &session_cookie)
        .call()
        .unwrap();
    assert_eq!(notes_res.status(), 200);

    server_handle.stop();
}

#[test]
fn test_server_tls_initialization() {
    let tmp = tempdir().unwrap();
    let notes_dir = tmp.path().join("notes");
    let db_path = tmp.path().join("test_tls.db");
    let backup_dir = tmp.path().join("backups");
    let tls_dir = tmp.path().join("tls");
    fs::create_dir_all(&notes_dir).unwrap();
    fs::create_dir_all(&tls_dir).unwrap();

    let cert_path = tls_dir.join("server.crt");
    let key_path = tls_dir.join("server.key");

    let config = ServerConfig {
        notes_dir: notes_dir.clone(),
        assets_dir: notes_dir.parent().unwrap_or(&notes_dir).join("assets"),
        db_path,
        backup_dir,
        port: 18960,
        bind_address: "127.0.0.1".to_string(),
        llm_config: LlmConfig::default(),
        permission_config: PermissionConfig::default(),
        auth_config: auth::AuthConfig::default(),
        enable_tls: true,
        tls_cert_path: Some(cert_path.clone()),
        tls_key_path: Some(key_path.clone()),
        reject_public_networks: true,
    };

    let server_handle = start_server_with_config(config).expect("TLS server should start");
    assert!(server_handle.is_tls());
    assert!(server_handle.primary_url().starts_with("https://"));
    assert!(cert_path.exists());
    assert!(key_path.exists());

    let agent = ureq::AgentBuilder::new()
        .tls_config(Arc::new(notesplusplus_core::agent::client::build_insecure_tls_client_config()))
        .build();
    let ping_res = agent
        .get(&format!("https://127.0.0.1:{}/api/ping", server_handle.port()))
        .call()
        .expect("HTTPS ping request should succeed");
    assert_eq!(ping_res.status(), 200);

    server_handle.stop();
}

#[test]
fn test_ip_classification_private_and_public() {
    use std::net::IpAddr;

    // IPv4 Loopback
    assert!(is_private_or_local_ip(&"127.0.0.1".parse::<IpAddr>().unwrap()));
    assert!(is_private_or_local_ip(&"127.255.0.1".parse::<IpAddr>().unwrap()));
    assert!(!is_public_ip(&"127.0.0.1".parse::<IpAddr>().unwrap()));

    // RFC 1918 10.0.0.0/8
    assert!(is_private_or_local_ip(&"10.0.0.1".parse::<IpAddr>().unwrap()));
    assert!(is_private_or_local_ip(&"10.255.255.255".parse::<IpAddr>().unwrap()));
    assert!(!is_public_ip(&"10.1.2.3".parse::<IpAddr>().unwrap()));

    // RFC 1918 172.16.0.0/12
    assert!(is_private_or_local_ip(&"172.16.0.1".parse::<IpAddr>().unwrap()));
    assert!(is_private_or_local_ip(&"172.31.255.254".parse::<IpAddr>().unwrap()));
    assert!(!is_private_or_local_ip(&"172.15.255.255".parse::<IpAddr>().unwrap()));
    assert!(!is_private_or_local_ip(&"172.32.0.1".parse::<IpAddr>().unwrap()));
    assert!(is_public_ip(&"172.32.0.1".parse::<IpAddr>().unwrap()));

    // RFC 1918 192.168.0.0/16
    assert!(is_private_or_local_ip(&"192.168.0.1".parse::<IpAddr>().unwrap()));
    assert!(is_private_or_local_ip(&"192.168.1.100".parse::<IpAddr>().unwrap()));
    assert!(!is_private_or_local_ip(&"192.169.1.1".parse::<IpAddr>().unwrap()));
    assert!(is_public_ip(&"192.169.1.1".parse::<IpAddr>().unwrap()));

    // RFC 3927 Link-local 169.254.0.0/16
    assert!(is_private_or_local_ip(&"169.254.1.1".parse::<IpAddr>().unwrap()));

    // RFC 6598 CGNAT 100.64.0.0/10
    assert!(is_private_or_local_ip(&"100.64.0.1".parse::<IpAddr>().unwrap()));
    assert!(is_private_or_local_ip(&"100.127.255.255".parse::<IpAddr>().unwrap()));
    assert!(!is_private_or_local_ip(&"100.128.0.1".parse::<IpAddr>().unwrap()));
    assert!(is_public_ip(&"100.128.0.1".parse::<IpAddr>().unwrap()));

    // Public IPv4 Addresses
    assert!(is_public_ip(&"8.8.8.8".parse::<IpAddr>().unwrap()));
    assert!(is_public_ip(&"1.1.1.1".parse::<IpAddr>().unwrap()));
    assert!(is_public_ip(&"93.184.216.34".parse::<IpAddr>().unwrap()));

    // IPv6 Loopback & Unspecified
    assert!(is_private_or_local_ip(&"::1".parse::<IpAddr>().unwrap()));
    assert!(is_private_or_local_ip(&"::".parse::<IpAddr>().unwrap()));

    // IPv6 Unique Local Addresses (fc00::/7)
    assert!(is_private_or_local_ip(&"fc00::1".parse::<IpAddr>().unwrap()));
    assert!(is_private_or_local_ip(&"fd12:3456:789a::1".parse::<IpAddr>().unwrap()));

    // IPv6 Link-Local (fe80::/10)
    assert!(is_private_or_local_ip(&"fe80::1".parse::<IpAddr>().unwrap()));

    // IPv4-mapped IPv6
    assert!(is_private_or_local_ip(&"::ffff:192.168.1.1".parse::<IpAddr>().unwrap()));
    assert!(is_public_ip(&"::ffff:8.8.8.8".parse::<IpAddr>().unwrap()));

    // Public IPv6
    assert!(is_public_ip(&"2607:f8b0:4005:805::200e".parse::<IpAddr>().unwrap()));
}

#[test]
fn test_server_reject_public_networks_toggle() {
    let tmp = tempdir().unwrap();
    let notes_dir = tmp.path().join("notes");
    let db_path = tmp.path().join("test_rej.db");
    let backup_dir = tmp.path().join("backups");
    fs::create_dir_all(&notes_dir).unwrap();

    let config = ServerConfig {
        notes_dir,
        db_path,
        backup_dir,
        port: 18980,
        reject_public_networks: true,
        ..Default::default()
    };

    let server_handle = start_server_with_config(config).expect("Server should start");
    assert!(server_handle.context().reject_public_networks());

    // Dynamic update
    server_handle.context().set_reject_public_networks(false);
    assert!(!server_handle.context().reject_public_networks());

    server_handle.context().set_reject_public_networks(true);
    assert!(server_handle.context().reject_public_networks());

    server_handle.stop();
}


#[test]
fn test_link_page_widget_web_assets() {
    use notesplusplus_core::server::web_assets::{INDEX_HTML, APP_JS, STYLE_CSS, STORES_LINK_STORE_JS};

    // 1. Verify index.html contains Link buttons and Link Dialog modal overlay
    assert!(INDEX_HTML.contains("openLinkDialog"));
    assert!(INDEX_HTML.contains("link-modal-overlay"));
    assert!(INDEX_HTML.contains("link-search-box"));
    assert!(INDEX_HTML.contains("link-pages-list"));
    assert!(INDEX_HTML.contains("linkDisplayText"));
    assert!(INDEX_HTML.contains("formattedLinkPreview"));
    assert!(INDEX_HTML.contains("confirmLinkInsert"));

    // 2. Verify link store and app.js contain link dialog state, computeds, and shortcut handlers
    assert!(APP_JS.contains("useLinkStore"));
    assert!(APP_JS.contains("openLinkDialog"));
    assert!(STORES_LINK_STORE_JS.contains("openLinkModal"));
    assert!(STORES_LINK_STORE_JS.contains("linkSearchQuery"));
    assert!(STORES_LINK_STORE_JS.contains("filteredLinkPages"));
    assert!(STORES_LINK_STORE_JS.contains("formattedLinkPreview"));
    assert!(STORES_LINK_STORE_JS.contains("confirmLinkInsert"));
    assert!(STORES_LINK_STORE_JS.contains("handleLinkKeydown"));
    assert!(STORES_LINK_STORE_JS.contains("selectLinkTarget"));
    assert!(STORES_LINK_STORE_JS.contains("isExternalUrl"));
    assert!(STORES_LINK_STORE_JS.contains("computedCustomFilename"));

    // 3. Verify style.css contains link modal classes
    assert!(STYLE_CSS.contains(".link-modal-card"));
    assert!(STYLE_CSS.contains(".link-search-box"));
    assert!(STYLE_CSS.contains(".link-pages-list"));
    assert!(STYLE_CSS.contains(".link-page-item"));
    assert!(STYLE_CSS.contains(".link-preview-container"));
}

#[test]
fn test_ai_model_selection_web_assets() {
    use notesplusplus_core::server::web_assets::{INDEX_HTML, APP_JS, STYLE_CSS, STORES_AI_JS};

    // 1. Verify index.html contains model selection UI elements
    assert!(INDEX_HTML.contains("ai-model-select"));
    assert!(INDEX_HTML.contains("availableModels"));
    assert!(INDEX_HTML.contains("onModelSelect"));
    assert!(INDEX_HTML.contains("aiConfig.model"));

    // Verify index.html does NOT leak server address, api key, token inputs, or self-signed cert option
    assert!(!INDEX_HTML.contains("aiConfig.endpoint"));
    assert!(!INDEX_HTML.contains("aiConfig.apiKey"));
    assert!(!INDEX_HTML.contains("Endpoint URL"));
    assert!(!INDEX_HTML.contains("API Key"));
    assert!(!INDEX_HTML.contains("allow_self_signed"));
    assert!(!INDEX_HTML.contains("Accept Self-Signed"));

    // 2. Verify app.js imports and uses AI store
    assert!(APP_JS.contains("useAiStore"));
    assert!(STORES_AI_JS.contains("availableModels"));
    assert!(STORES_AI_JS.contains("fetchAvailableModels"));
    assert!(STORES_AI_JS.contains("onModelSelect"));
    assert!(STORES_AI_JS.contains("isCurrentModelInList"));

    // Verify AI store contains provider, model fetching, system prompt state & methods
    assert!(STORES_AI_JS.contains("/api/ai/models"));
    assert!(STORES_AI_JS.contains("system_prompt"));
    assert!(STORES_AI_JS.contains("provider"));

    // Verify store does not expose or send server endpoints, tokens, or self-signed cert flags
    assert!(!STORES_AI_JS.contains("aiConfig.endpoint"));
    assert!(!STORES_AI_JS.contains("aiConfig.apiKey"));
    assert!(!STORES_AI_JS.contains("allow_self_signed"));

    // 3. Verify style.css contains styling for the model select
    assert!(STYLE_CSS.contains(".ai-model-select"));
}

#[test]
fn test_gallery_view_and_rendered_cards_web_assets() {
    use notesplusplus_core::server::web_assets::{INDEX_HTML, STYLE_CSS, STORES_NOTES_JS};

    // 1. Verify index.html contains Gallery view mode tab button, container, card grid, color pill, and note ID label
    assert!(INDEX_HTML.contains("viewMode === 'gallery'"));
    assert!(INDEX_HTML.contains("gallery-container"));
    assert!(INDEX_HTML.contains("gallery-search-input"));
    assert!(INDEX_HTML.contains("note-group-section"));
    assert!(INDEX_HTML.contains("notes-card-grid"));
    assert!(INDEX_HTML.contains("note-card"));
    assert!(INDEX_HTML.contains("mini-doc-preview"));
    assert!(INDEX_HTML.contains("note-color-bar"));
    assert!(INDEX_HTML.contains("note-id-label"));
    assert!(INDEX_HTML.contains("toggleGroupCollapse"));
    assert!(INDEX_HTML.contains("selectNote"));

    // 2. Verify notes store contains group tree state, fetching, collapse toggling, and card navigation
    assert!(STORES_NOTES_JS.contains("groupTree"));
    assert!(STORES_NOTES_JS.contains("fetchGroupTree"));
    assert!(STORES_NOTES_JS.contains("toggleGroupCollapse"));
    assert!(STORES_NOTES_JS.contains("isGroupCollapsed"));
    assert!(STORES_NOTES_JS.contains("filteredGroupTree"));
    assert!(STORES_NOTES_JS.contains("selectNote"));
    assert!(STORES_NOTES_JS.contains("/api/tree"));

    // 3. Verify style.css contains styling for gallery, grid, note-card, mini preview, and footer
    assert!(STYLE_CSS.contains(".gallery-container"));
    assert!(STYLE_CSS.contains(".notes-card-grid"));
    assert!(STYLE_CSS.contains(".note-card"));
    assert!(STYLE_CSS.contains(".mini-doc-preview"));
    assert!(STYLE_CSS.contains(".card-footer"));
    assert!(STYLE_CSS.contains(".note-color-bar"));
    assert!(STYLE_CSS.contains(".note-id-label"));

    // 4. Verify search UI enhancements
    assert!(INDEX_HTML.contains("placeholder=\"Search notes by title or content...\""));
    assert!(INDEX_HTML.contains("btn-clear-search"));
    assert!(INDEX_HTML.contains("gallery-search-results"));
    assert!(INDEX_HTML.contains("search-note-card"));
    assert!(INDEX_HTML.contains("search-highlight-bar"));
    assert!(INDEX_HTML.contains("clearDocumentHighlight"));

    assert!(STORES_NOTES_JS.contains("searchResults"));
    assert!(STORES_NOTES_JS.contains("isSearching"));
    assert!(STORES_NOTES_JS.contains("activeSearchTerm"));
    assert!(STORES_NOTES_JS.contains("performSearch"));
    assert!(STORES_NOTES_JS.contains("clearSearch"));
    assert!(STORES_NOTES_JS.contains("selectSearchResultNote"));
    assert!(STORES_NOTES_JS.contains("clearDocumentHighlight"));

    assert!(STYLE_CSS.contains(".btn-clear-search"));
    assert!(STYLE_CSS.contains(".search-results-header"));
    assert!(STYLE_CSS.contains(".search-note-card"));
    assert!(STYLE_CSS.contains(".card-group-tag"));
    assert!(STYLE_CSS.contains(".search-highlight-bar"));
    assert!(STYLE_CSS.contains(".search-match"));
}

#[test]
fn test_list_all_notes_json_metadata_and_filtering() {
    let tmp = tempdir().unwrap();
    let notes_dir = tmp.path().join("notes");
    fs::create_dir_all(&notes_dir).unwrap();

    fs::write(notes_dir.join("architecture.adoc"), "= System Architecture\nHigh level architecture diagram and components.").unwrap();
    fs::write(notes_dir.join("meeting-notes.adoc"), "= Meeting Notes\nDiscussion on deployment and release roadmap.").unwrap();

    // Without query: returns all notes
    let all_json = list_all_notes_json(&notes_dir, None);
    let all_notes: Vec<serde_json::Value> = serde_json::from_str(&all_json).unwrap();
    assert_eq!(all_notes.len(), 2);
    let filenames: Vec<&str> = all_notes.iter().filter_map(|n| n["filename"].as_str()).collect();
    assert!(filenames.contains(&"meeting-notes.adoc"));
    assert!(filenames.contains(&"architecture.adoc"));

    // With query matching title/snippet
    let filtered_json = list_all_notes_json(&notes_dir, Some("Architecture"));
    let filtered_notes: Vec<serde_json::Value> = serde_json::from_str(&filtered_json).unwrap();
    assert_eq!(filtered_notes.len(), 1);
    assert_eq!(filtered_notes[0]["filename"], "architecture.adoc");
    assert_eq!(filtered_notes[0]["title"], "System Architecture");
}

#[test]
fn test_security_asset_path_isolation() {
    let tmp = tempdir().unwrap();
    let notes_dir = tmp.path().join("notes");
    let db_path = tmp.path().join("test_security.db");
    let backup_dir = tmp.path().join("backups");
    fs::create_dir_all(&notes_dir).unwrap();

    // Put a .adoc file in the notes directory
    fs::write(notes_dir.join("secret.adoc"), "= Secret\nConfidential content.").unwrap();

    let server_handle = start_server_full(
        notes_dir.clone(),
        db_path,
        backup_dir,
        18990,
        None,
        None,
    ).expect("Server should start");
    let port = server_handle.port();

    // /assets/secret.adoc should NOT serve the note (notes are in notes/ subdir now)
    let res = ureq::get(&format!("http://127.0.0.1:{}/assets/secret.adoc", port)).call();
    match res {
        Ok(resp) => panic!("Expected 404 for assets/secret.adoc, got {}", resp.status()),
        Err(ureq::Error::Status(code, _)) => assert_eq!(code, 404, "assets/ should not serve .adoc files from notes subdir"),
        Err(e) => panic!("Unexpected error: {}", e),
    }

    // /raw/secret.adoc without auth should return the login page (SPA), not raw content
    let res_raw = ureq::get(&format!("http://127.0.0.1:{}/raw/secret.adoc", port)).call().unwrap();
    assert_eq!(res_raw.status(), 200);
    let body = res_raw.into_string().unwrap();
    assert!(body.contains("Notes Plus"), "unauthenticated /raw/ should serve login page, not note content");

    // /api/notes without auth should return 401
    let res_api = ureq::get(&format!("http://127.0.0.1:{}/api/notes", port)).call();
    match res_api {
        Ok(resp) => panic!("Expected 401 for unauthenticated /api/notes, got {}", resp.status()),
        Err(ureq::Error::Status(code, _)) => assert_eq!(code, 401),
        Err(e) => panic!("Unexpected error: {}", e),
    }

    // /page/test without auth should return the login page
    let res_page = ureq::get(&format!("http://127.0.0.1:{}/page/test", port)).call().unwrap();
    assert_eq!(res_page.status(), 200);
    let page_body = res_page.into_string().unwrap();
    assert!(page_body.contains("Notes Plus"), "unauthenticated /page/ should serve login page");

    // Path traversal attempt should be blocked (use raw TCP to avoid URL normalization)
    {
        use std::io::{Read, Write as IoWrite};
        let mut tcp = std::net::TcpStream::connect(format!("127.0.0.1:{}", port)).unwrap();
        tcp.write_all(b"GET /assets/../../etc/passwd HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n").unwrap();
        let mut resp = String::new();
        tcp.read_to_string(&mut resp).unwrap();
        assert!(resp.starts_with("HTTP/1.1 403"), "path traversal should return 403, got: {}", &resp[..50.min(resp.len())]);
    }

    // /api/ping should not leak TLS or auth status
    let res_ping = ureq::get(&format!("http://127.0.0.1:{}/api/ping", port)).call().unwrap();
    assert_eq!(res_ping.status(), 200);
    let ping_json: serde_json::Value = res_ping.into_json().unwrap();
    assert!(ping_json.get("ok").is_some(), "ping should have 'ok' field");
    assert!(ping_json.get("is_tls").is_none(), "ping should not leak TLS status");
    assert!(ping_json.get("auth_enabled").is_none(), "ping should not leak auth status");

    // /api/auth/config without auth should not leak username
    let res_cfg = ureq::get(&format!("http://127.0.0.1:{}/api/auth/config", port)).call().unwrap();
    assert_eq!(res_cfg.status(), 200);
    let cfg_json: serde_json::Value = res_cfg.into_json().unwrap();
    assert!(cfg_json.get("basic_username").is_none(), "auth config should not leak username");
    assert!(cfg_json.get("has_password").is_none(), "auth config should not leak password existence");

    // Verify security headers are present
    let res_headers = ureq::get(&format!("http://127.0.0.1:{}/", port)).call().unwrap();
    assert_eq!(res_headers.header("X-Content-Type-Options").unwrap(), "nosniff");
    assert_eq!(res_headers.header("X-Frame-Options").unwrap(), "DENY");
    assert_eq!(res_headers.header("Referrer-Policy").unwrap(), "no-referrer");

    server_handle.stop();
}

#[test]
fn test_logout_flow_and_session_invalidation() {
    let tmp = tempdir().unwrap();
    let notes_dir = tmp.path().join("notes");
    let db_path = tmp.path().join("test_logout.db");
    let backup_dir = tmp.path().join("backups");
    fs::create_dir_all(&notes_dir).unwrap();

    let server_handle = start_server_full(
        notes_dir.clone(),
        db_path,
        backup_dir,
        18992,
        None,
        None,
    ).expect("Server should start");
    let port = server_handle.port();

    // 1. Initiate challenge
    let init_res = ureq::post(&format!("http://127.0.0.1:{}/api/auth/code/initiate", port))
        .call()
        .unwrap();
    assert_eq!(init_res.status(), 200);
    let init_json: serde_json::Value = init_res.into_json().unwrap();
    let challenge_id = init_json["challenge_id"].as_str().unwrap().to_string();

    // 2. Approve challenge in-process (as native GUI does)
    server_handle.context().auth_challenges.approve_challenge(&challenge_id);
    server_handle.context().clear_auth_challenge();

    // 3. Poll challenge status to receive session
    let status_res = ureq::get(&format!("http://127.0.0.1:{}/api/auth/code/status?challenge_id={}", port, challenge_id))
        .call()
        .unwrap();
    assert_eq!(status_res.status(), 200);
    let set_cookie_hdr = status_res.header("Set-Cookie").unwrap().to_string();
    let status_json: serde_json::Value = status_res.into_json().unwrap();
    assert_eq!(status_json["status"], "approved");
    let session_id = status_json["session_id"].as_str().unwrap().to_string();

    // 4. Verify accessing protected API with session cookie succeeds
    let notes_res = ureq::get(&format!("http://127.0.0.1:{}/api/notes", port))
        .set("Cookie", &set_cookie_hdr)
        .call()
        .unwrap();
    assert_eq!(notes_res.status(), 200);

    // 5. Call POST /api/auth/logout
    let logout_res = ureq::post(&format!("http://127.0.0.1:{}/api/auth/logout", port))
        .set("Cookie", &set_cookie_hdr)
        .call()
        .unwrap();
    assert_eq!(logout_res.status(), 200);
    let logout_cookie_hdr = logout_res.header("Set-Cookie").unwrap();
    assert!(logout_cookie_hdr.contains("Max-Age=0") || logout_cookie_hdr.contains("notesplus_session="));

    // 6. Verify subsequent requests using the old session cookie are rejected with 401
    let reject_res = ureq::get(&format!("http://127.0.0.1:{}/api/notes", port))
        .set("Cookie", &set_cookie_hdr)
        .call();
    match reject_res {
        Ok(resp) => panic!("Expected 401 after logout, got {}", resp.status()),
        Err(ureq::Error::Status(code, _)) => assert_eq!(code, 401),
        Err(e) => panic!("Unexpected error: {}", e),
    }

    // 7. Verify subsequent requests using the old session ID as Bearer token are also rejected
    let reject_bearer = ureq::get(&format!("http://127.0.0.1:{}/api/notes", port))
        .set("Authorization", &format!("Bearer {}", session_id))
        .call();
    match reject_bearer {
        Ok(resp) => panic!("Expected 401 for revoked Bearer token, got {}", resp.status()),
        Err(ureq::Error::Status(code, _)) => assert_eq!(code, 401),
        Err(e) => panic!("Unexpected error: {}", e),
    }

    server_handle.stop();
}

#[test]
fn test_web_ui_logout_button_assets() {
    use notesplusplus_core::server::web_assets::{INDEX_HTML, APP_JS, STYLE_CSS, STORES_AUTH_JS};

    // 1. Verify index.html contains logout button and action bindings
    assert!(INDEX_HTML.contains("@click=\"logout\""));
    assert!(INDEX_HTML.contains("btn-logout"));
    assert!(INDEX_HTML.contains("Logout"));

    // 2. Verify app.js imports auth store and auth store exports logout handler
    assert!(APP_JS.contains("useAuthStore"));
    assert!(STORES_AUTH_JS.contains("logout"));

    // 3. Verify style.css defines styling for logout button
    assert!(STYLE_CSS.contains(".btn-logout"));
}

#[test]
fn test_rate_limiting_on_auth_endpoint() {
    let tmp = tempdir().unwrap();
    let notes_dir = tmp.path().join("notes");
    let db_path = tmp.path().join("test_rate_limit.db");
    let backup_dir = tmp.path().join("backups");
    fs::create_dir_all(&notes_dir).unwrap();

    let server_handle = start_server_full(
        notes_dir.clone(),
        db_path,
        backup_dir,
        18993,
        None,
        None,
    ).expect("Server should start");
    let port = server_handle.port();

    // Send 10 allowed initiate requests
    for _ in 0..10 {
        let res = ureq::post(&format!("http://127.0.0.1:{}/api/auth/code/initiate", port))
            .call()
            .unwrap();
        assert_eq!(res.status(), 200);
    }

    // The 11th request from the same IP should be blocked by rate limiter with 429
    let blocked_res = ureq::post(&format!("http://127.0.0.1:{}/api/auth/code/initiate", port))
        .call();
    match blocked_res {
        Ok(resp) => panic!("Expected 429 Too Many Requests, got {}", resp.status()),
        Err(ureq::Error::Status(code, resp)) => {
            assert_eq!(code, 429);
            assert!(resp.header("Retry-After").is_some());
            let body: serde_json::Value = resp.into_json().unwrap();
            assert_eq!(body["ok"], false);
            assert!(body["error"].as_str().unwrap().contains("Too many authentication requests"));
            assert!(body["retry_after"].as_u64().unwrap() >= 1);
        }
        Err(e) => panic!("Unexpected error: {}", e),
    }

    server_handle.stop();
}

#[test]
fn test_configurable_session_expiration_and_remaining_time() {
    let tmp = tempdir().unwrap();
    let notes_dir = tmp.path().join("notes");
    let db_path = tmp.path().join("test_session_expiry.db");
    let backup_dir = tmp.path().join("backups");
    fs::create_dir_all(&notes_dir).unwrap();

    let server_handle = start_server_full(
        notes_dir.clone(),
        db_path,
        backup_dir,
        18994,
        None,
        None,
    ).expect("Server should start");
    let port = server_handle.port();

    // Configure session duration to 7200 seconds (2 hours)
    server_handle.context().set_session_expiry_secs(7200);
    assert_eq!(server_handle.context().session_expiry_secs(), 7200);

    // Initiate challenge
    let init_res = ureq::post(&format!("http://127.0.0.1:{}/api/auth/code/initiate", port))
        .call()
        .unwrap();
    let init_json: serde_json::Value = init_res.into_json().unwrap();
    let challenge_id = init_json["challenge_id"].as_str().unwrap().to_string();

    // Approve challenge in-process (as native GUI does)
    server_handle.context().auth_challenges.approve_challenge(&challenge_id);
    server_handle.context().clear_auth_challenge();

    // Poll status and verify remaining_secs and expires_at are reported
    let status_res = ureq::get(&format!("http://127.0.0.1:{}/api/auth/code/status?challenge_id={}", port, challenge_id))
        .call()
        .unwrap();
    assert_eq!(status_res.status(), 200);
    let set_cookie_hdr = status_res.header("Set-Cookie").unwrap().to_string();
    let status_json: serde_json::Value = status_res.into_json().unwrap();
    assert_eq!(status_json["status"], "approved");
    let remaining_secs = status_json["remaining_secs"].as_u64().unwrap();
    assert!(remaining_secs >= 7190 && remaining_secs <= 7200);
    let expires_at = status_json["expires_at"].as_u64().unwrap();

    // Check /api/auth/config returns authenticated session info with remaining_secs
    let cfg_res = ureq::get(&format!("http://127.0.0.1:{}/api/auth/config", port))
        .set("Cookie", &set_cookie_hdr)
        .call()
        .unwrap();
    assert_eq!(cfg_res.status(), 200);
    let cfg_json: serde_json::Value = cfg_res.into_json().unwrap();
    assert_eq!(cfg_json["authenticated"], true);
    assert_eq!(cfg_json["expires_at"], expires_at);
    assert!(cfg_json["remaining_secs"].as_u64().unwrap() > 0);

    // Check /api/auth/whoami returns session details with remaining_secs
    let whoami_res = ureq::get(&format!("http://127.0.0.1:{}/api/auth/whoami", port))
        .set("Cookie", &set_cookie_hdr)
        .call()
        .unwrap();
    assert_eq!(whoami_res.status(), 200);
    let whoami_json: serde_json::Value = whoami_res.into_json().unwrap();
    assert_eq!(whoami_json["authenticated"], true);
    assert_eq!(whoami_json["expires_at"], expires_at);
    assert!(whoami_json["remaining_secs"].as_u64().unwrap() > 0);

    server_handle.stop();
}

#[test]
fn test_web_ui_session_timer_assets() {
    use notesplusplus_core::server::web_assets::{INDEX_HTML, APP_JS, STYLE_CSS, STORES_AUTH_JS};

    // 1. Verify index.html contains session chip and remaining text
    assert!(INDEX_HTML.contains("session-chip"));
    assert!(INDEX_HTML.contains("sessionRemainingText"));
    assert!(INDEX_HTML.contains("session-time"));

    // 2. Verify app.js imports auth store and store exports session timer state
    assert!(APP_JS.contains("useAuthStore"));
    assert!(STORES_AUTH_JS.contains("sessionRemainingText"));

    // 3. Verify style.css defines styling for session chip
    assert!(STYLE_CSS.contains(".session-chip"));
    assert!(STYLE_CSS.contains(".session-time"));
}

#[test]
fn test_web_ui_responsive_header_and_account_dropdown_assets() {
    use notesplusplus_core::server::web_assets::{INDEX_HTML, APP_JS, STYLE_CSS};

    // 1. Verify index.html contains structured nav groups and account dropdown
    assert!(INDEX_HTML.contains("nav-actions-group"));
    assert!(INDEX_HTML.contains("nav-account-group"));
    assert!(INDEX_HTML.contains("account-dropdown-menu"));
    assert!(INDEX_HTML.contains("account-dropdown-wrapper"));
    assert!(INDEX_HTML.contains("view-mode-tabs"));
    assert!(INDEX_HTML.contains("btn-account"));

    // 2. Verify app.js exports showAccountMenu
    assert!(APP_JS.contains("showAccountMenu"));

    // 3. Verify style.css contains account dropdown and responsive queries
    assert!(STYLE_CSS.contains(".account-dropdown-menu"));
    assert!(STYLE_CSS.contains(".btn-account"));
    assert!(STYLE_CSS.contains("@media (max-width: 1080px)"));
    assert!(STYLE_CSS.contains("@media (max-width: 860px)"));
    assert!(STYLE_CSS.contains("@media (max-width: 640px)"));
}

#[test]
fn test_import_from_url_and_file_web_assets() {
    use notesplusplus_core::server::web_assets::{INDEX_HTML, APP_JS, STYLE_CSS, STORES_IMPORT_STORE_JS};

    // 1. Verify index.html contains import actions, URL inputs, server file path inputs, and file picker
    assert!(INDEX_HTML.contains("Fetch URL"));
    assert!(INDEX_HTML.contains("Choose File"));
    assert!(INDEX_HTML.contains("toggleUrlInput"));
    assert!(INDEX_HTML.contains("triggerFilePicker"));
    assert!(INDEX_HTML.contains("toggleFileInput"));
    assert!(INDEX_HTML.contains("fetchUrlContent"));
    assert!(INDEX_HTML.contains("loadServerFile"));
    assert!(INDEX_HTML.contains("pasteClipboard"));
    assert!(INDEX_HTML.contains("fileInputRef"));
    assert!(INDEX_HTML.contains("showUrlInput"));
    assert!(INDEX_HTML.contains("showFileInput"));
    assert!(INDEX_HTML.contains("isDraggingFile"));
    assert!(INDEX_HTML.contains("import-textarea-wrapper"));

    // 2. Verify app.js and importStore define handlers, state, and API routes
    assert!(APP_JS.contains("useImportStore"));
    assert!(STORES_IMPORT_STORE_JS.contains("importUrl"));
    assert!(STORES_IMPORT_STORE_JS.contains("showUrlInput"));
    assert!(STORES_IMPORT_STORE_JS.contains("isFetchingUrl"));
    assert!(STORES_IMPORT_STORE_JS.contains("fetchUrlContent"));
    assert!(STORES_IMPORT_STORE_JS.contains("onFileSelect"));
    assert!(STORES_IMPORT_STORE_JS.contains("onFileDrop"));
    assert!(STORES_IMPORT_STORE_JS.contains("loadServerFile"));
    assert!(STORES_IMPORT_STORE_JS.contains("pasteClipboard"));

    // 3. Verify style.css contains import source action and dropzone classes
    assert!(STYLE_CSS.contains(".import-source-header"));
    assert!(STYLE_CSS.contains(".import-source-actions"));
    assert!(STYLE_CSS.contains(".btn-import-source"));
    assert!(STYLE_CSS.contains(".import-input-card"));
    assert!(STYLE_CSS.contains(".import-textarea-wrapper"));
    assert!(STYLE_CSS.contains(".drag-drop-overlay"));
}

#[test]
fn test_fetch_url_and_read_file_endpoints() {
    let tmp = tempdir().unwrap();
    let notes_dir = tmp.path().join("notes");
    let db_path = tmp.path().join("test_import_endpoints.db");
    let backup_dir = tmp.path().join("backups");
    fs::create_dir_all(&notes_dir).unwrap();

    fs::write(notes_dir.join("imported-sample.adoc"), "= Sample Imported Note\nThis is a test note for import.").unwrap();

    let server_handle = start_server_full(
        notes_dir.clone(),
        db_path,
        backup_dir,
        18995,
        None,
        None,
    ).expect("Server should start");
    let port = server_handle.port();

    // 1. Initiate challenge
    let init_res = ureq::post(&format!("http://127.0.0.1:{}/api/auth/code/initiate", port))
        .call()
        .unwrap();
    let init_json: serde_json::Value = init_res.into_json().unwrap();
    let challenge_id = init_json["challenge_id"].as_str().unwrap().to_string();

    // 2. Approve challenge in-process (as native GUI does)
    server_handle.context().auth_challenges.approve_challenge(&challenge_id);
    server_handle.context().clear_auth_challenge();

    // 3. Poll status for session
    let status_res = ureq::get(&format!("http://127.0.0.1:{}/api/auth/code/status?challenge_id={}", port, challenge_id))
        .call()
        .unwrap();
    let set_cookie_hdr = status_res.header("Set-Cookie").unwrap().to_string();

    // 4. Test POST /api/ai/fetch_url with blocked host (localhost)
    let blocked_res = ureq::post(&format!("http://127.0.0.1:{}/api/ai/fetch_url", port))
        .set("Cookie", &set_cookie_hdr)
        .set("Content-Type", "application/json")
        .send_string(r#"{"url": "http://127.0.0.1:8080/test"}"#);
    match blocked_res {
        Ok(resp) => {
            let json: serde_json::Value = resp.into_json().unwrap();
            assert_eq!(json["ok"], false);
        }
        Err(ureq::Error::Status(code, resp)) => {
            assert_eq!(code, 400);
            let json: serde_json::Value = resp.into_json().unwrap();
            assert_eq!(json["ok"], false);
            assert!(json["error"].as_str().unwrap().contains("blocked"));
        }
        Err(e) => panic!("Unexpected error: {}", e),
    }

    // 5. Test POST /api/ai/fetch_url with missing URL parameter
    let empty_url_res = ureq::post(&format!("http://127.0.0.1:{}/api/ai/fetch_url", port))
        .set("Cookie", &set_cookie_hdr)
        .set("Content-Type", "application/json")
        .send_string(r#"{"url": ""}"#);
    match empty_url_res {
        Ok(resp) => panic!("Expected 400, got {}", resp.status()),
        Err(ureq::Error::Status(code, _)) => assert_eq!(code, 400),
        Err(e) => panic!("Unexpected error: {}", e),
    }

    // 6. Test POST /api/ai/read_file with directory traversal (should be forbidden)
    let traversal_res = ureq::post(&format!("http://127.0.0.1:{}/api/ai/read_file", port))
        .set("Cookie", &set_cookie_hdr)
        .set("Content-Type", "application/json")
        .send_string(r#"{"file_path": "../../etc/passwd"}"#);
    match traversal_res {
        Ok(resp) => panic!("Expected 403, got {}", resp.status()),
        Err(ureq::Error::Status(code, _)) => assert_eq!(code, 403),
        Err(e) => panic!("Unexpected error: {}", e),
    }

    // 7. Test POST /api/ai/read_file with valid file within notes directory
    let file_path = notes_dir.join("imported-sample.adoc").to_str().unwrap().to_string();
    let valid_file_res = ureq::post(&format!("http://127.0.0.1:{}/api/ai/read_file", port))
        .set("Cookie", &set_cookie_hdr)
        .set("Content-Type", "application/json")
        .send_string(&format!(r#"{{"file_path": "{}"}}"#, file_path))
        .unwrap();
    assert_eq!(valid_file_res.status(), 200);
    let valid_json: serde_json::Value = valid_file_res.into_json().unwrap();
    assert_eq!(valid_json["ok"], true);
    assert!(valid_json["content"].as_str().unwrap().contains("Sample Imported Note"));

    // 8. Test POST /api/ai/preprocess_html
    let raw_html = "<html><head><script>alert(1);</script><style>body{color:red;}</style></head><body><h1>Web Title</h1><p>Paragraph with <b>bold</b> text.</p></body></html>";
    let preprocess_res = ureq::post(&format!("http://127.0.0.1:{}/api/ai/preprocess_html", port))
        .set("Cookie", &set_cookie_hdr)
        .set("Content-Type", "application/json")
        .send_string(&serde_json::to_string(&serde_json::json!({ "html": raw_html })).unwrap())
        .unwrap();
    assert_eq!(preprocess_res.status(), 200);
    let prep_json: serde_json::Value = preprocess_res.into_json().unwrap();
    assert_eq!(prep_json["ok"], true);
    let content = prep_json["content"].as_str().unwrap();
    assert!(content.contains("# Web Title"));
    assert!(content.contains("Paragraph with **bold** text."));
    assert!(!content.contains("alert(1)"));
    assert!(!content.contains("body{color:red;}"));

    server_handle.stop();
}

#[test]
fn test_challenge_approve_endpoint_not_exposed_over_http() {
    let tmp = tempdir().unwrap();
    let notes_dir = tmp.path().join("notes");
    let db_path = tmp.path().join("test_auth_http_safety.db");
    let backup_dir = tmp.path().join("backups");
    fs::create_dir_all(&notes_dir).unwrap();

    let server_handle = start_server_full(
        notes_dir.clone(),
        db_path,
        backup_dir,
        18996,
        None,
        None,
    ).expect("Server should start");
    let port = server_handle.port();

    // 1. Initiate challenge
    let init_res = ureq::post(&format!("http://127.0.0.1:{}/api/auth/code/initiate", port))
        .call()
        .unwrap();
    let init_json: serde_json::Value = init_res.into_json().unwrap();
    let challenge_id = init_json["challenge_id"].as_str().unwrap().to_string();

    // 2. Attempting to approve or deny via HTTP fails / returns 404
    let approve_attempt = ureq::post(&format!("http://127.0.0.1:{}/api/auth/code/approve", port))
        .set("Content-Type", "application/json")
        .send_string(&format!(r#"{{"challenge_id": "{}"}}"#, challenge_id));
    match approve_attempt {
        Ok(resp) => panic!("Expected 404 for removed approve route, got {}", resp.status()),
        Err(ureq::Error::Status(code, _)) => assert_eq!(code, 404),
        Err(e) => panic!("Unexpected error: {}", e),
    }

    let deny_attempt = ureq::post(&format!("http://127.0.0.1:{}/api/auth/code/deny", port))
        .set("Content-Type", "application/json")
        .send_string(&format!(r#"{{"challenge_id": "{}"}}"#, challenge_id));
    match deny_attempt {
        Ok(resp) => panic!("Expected 404 for removed deny route, got {}", resp.status()),
        Err(ureq::Error::Status(code, _)) => assert_eq!(code, 404),
        Err(e) => panic!("Unexpected error: {}", e),
    }

    // 3. Status remains pending because HTTP attempts did nothing
    let status_res = ureq::get(&format!("http://127.0.0.1:{}/api/auth/code/status?challenge_id={}", port, challenge_id))
        .call()
        .unwrap();
    let status_json: serde_json::Value = status_res.into_json().unwrap();
    assert_eq!(status_json["status"], "pending");

    // 4. In-process approval (native GUI bridge) succeeds
    server_handle.context().auth_challenges.approve_challenge(&challenge_id);
    server_handle.context().clear_auth_challenge();

    let approved_res = ureq::get(&format!("http://127.0.0.1:{}/api/auth/code/status?challenge_id={}", port, challenge_id))
        .call()
        .unwrap();
    let approved_json: serde_json::Value = approved_res.into_json().unwrap();
    assert_eq!(approved_json["status"], "approved");

    server_handle.stop();
}


#[test]
fn test_composable_js_files_served() {
    let tmp = tempdir().unwrap();
    let notes_dir = tmp.path().join("notes");
    let db_path = tmp.path().join("test_composable.db");
    let backup_dir = tmp.path().join("backups");
    fs::create_dir_all(&notes_dir).unwrap();

    let server_handle = start_server_full(notes_dir.clone(), db_path, backup_dir, 18997, None, None).expect("Server should start");
    let port = server_handle.port();

    let assets = [
        "composables/utils.js", "stores/auth.js", "stores/theme.js",
        "stores/health.js", "stores/presentationStore.js",
        "stores/linkStore.js", "stores/ai.js", "stores/importStore.js",
        "stores/notes.js", "stores/editorStore.js", "stores/ui.js",
    ];

    for path in &assets {
        let res = ureq::get(&format!("http://127.0.0.1:{}/{}", port, path)).call().unwrap();
        assert_eq!(res.status(), 200, "Expected 200 for {}", path);
        let body = res.into_string().unwrap();
        assert!(body.len() > 10, "Expected non-trivial content for {}", path);
    }

    let app_js = ureq::get(&format!("http://127.0.0.1:{}/app.js", port)).call().unwrap().into_string().unwrap();
    assert!(app_js.contains("from '/composables/utils.js'") || app_js.contains("from './composables/utils.js'"), "app.js should import from composables/utils.js");

    server_handle.stop();
}

#[test]
fn test_render_svgbob_block() {
    let tmp = tempdir().unwrap();
    let notes_dir = tmp.path().join("notes");
    let db_path = tmp.path().join("test_svgbob.db");
    let backup_dir = tmp.path().join("backups");
    let assets_dir = tmp.path().join("assets");
    fs::create_dir_all(&notes_dir).unwrap();
    fs::create_dir_all(&assets_dir).unwrap();

    let config = ServerConfig {
        notes_dir: notes_dir.clone(),
        assets_dir,
        db_path,
        backup_dir,
        ..Default::default()
    };

    let server_handle = start_server_with_config(config).expect("Server should start");
    let port = server_handle.port();

    // Authenticate
    let sess = server_handle.context().session_store.create_session("admin", "code", 3600).unwrap();
    let session_cookie = format!("{}={}", notesplusplus_core::constants::SESSION_COOKIE_NAME, sess.id);

    let adoc = "[source,svgbob]\n----\n+---+\n| A |\n+---+\n----";
    let res = ureq::post(&format!("http://127.0.0.1:{}/api/render", port))
        .set("Cookie", &session_cookie)
        .send_json(json!({
            "content": adoc
        }))
        .unwrap();
    assert_eq!(res.status(), 200);
    let body = res.into_string().unwrap();
    assert!(body.contains("<svg"), "Expected SVG in rendered output, got: {}", &body[..500.min(body.len())]);
    assert!(!body.contains("<pre><code"), "Should NOT render as code block");

    server_handle.stop();
}

#[test]
fn test_groups_api_and_multisegment_notes() {
    let tmp = tempdir().unwrap();
    let notes_dir = tmp.path().join("notes");
    let db_path = tmp.path().join("test_groups.db");
    let backup_dir = tmp.path().join("backups");
    let assets_dir = tmp.path().join("assets");
    fs::create_dir_all(&notes_dir).unwrap();
    fs::create_dir_all(&assets_dir).unwrap();

    let config = ServerConfig {
        notes_dir: notes_dir.clone(),
        assets_dir,
        db_path,
        backup_dir,
        ..Default::default()
    };

    let server_handle = start_server_with_config(config).expect("Server should start");
    let port = server_handle.port();

    let sess = server_handle.context().session_store.create_session("admin", "code", 3600).unwrap();
    let session_cookie = format!("{}={}", notesplusplus_core::constants::SESSION_COOKIE_NAME, sess.id);

    // 1. Create group via POST /api/groups
    let create_group_res = ureq::post(&format!("http://127.0.0.1:{}/api/groups", port))
        .set("Cookie", &session_cookie)
        .send_json(json!({
            "name": "Projects",
            "parent": "Work"
        }))
        .unwrap();
    assert_eq!(create_group_res.status(), 200);

    // 2. List groups via GET /api/groups
    let list_groups_res = ureq::get(&format!("http://127.0.0.1:{}/api/groups", port))
        .set("Cookie", &session_cookie)
        .call()
        .unwrap();
    assert_eq!(list_groups_res.status(), 200);
    let groups_json: serde_json::Value = list_groups_res.into_json().unwrap();
    assert!(groups_json.as_array().unwrap().iter().any(|g| g["path"] == "Work/Projects"));

    // 3. Create nested note via PUT /api/notes/Work/Projects/Sprint.adoc
    let put_note_res = ureq::put(&format!("http://127.0.0.1:{}/api/notes/Work/Projects/Sprint.adoc", port))
        .set("Cookie", &session_cookie)
        .set("Content-Type", "text/plain")
        .send_string("= Sprint Plan\nNested note content.")
        .unwrap();
    assert_eq!(put_note_res.status(), 200);

    // 4. Read nested note via GET /api/notes/Work/Projects/Sprint.adoc
    let get_note_res = ureq::get(&format!("http://127.0.0.1:{}/api/notes/Work/Projects/Sprint.adoc", port))
        .set("Cookie", &session_cookie)
        .call()
        .unwrap();
    assert_eq!(get_note_res.status(), 200);
    assert!(get_note_res.into_string().unwrap().contains("Sprint Plan"));

    // 5. Update group note_sort via PUT /api/groups/Work/Projects
    let put_group_res = ureq::put(&format!("http://127.0.0.1:{}/api/groups/Work/Projects", port))
        .set("Cookie", &session_cookie)
        .send_json(json!({
            "note_sort": "name"
        }))
        .unwrap();
    assert_eq!(put_group_res.status(), 200);

    // 6. Verify updated sort in GET /api/groups
    let list_after_res = ureq::get(&format!("http://127.0.0.1:{}/api/groups", port))
        .set("Cookie", &session_cookie)
        .call()
        .unwrap();
    assert_eq!(list_after_res.status(), 200);
    let groups_after_json: serde_json::Value = list_after_res.into_json().unwrap();
    let updated_group = groups_after_json.as_array().unwrap().iter().find(|g| g["path"] == "Work/Projects").unwrap();
    assert_eq!(updated_group["note_sort"], "name");

    server_handle.stop();
}

#[test]
fn test_theme_api_endpoint() {
    let tmp = tempdir().unwrap();
    let notes_dir = tmp.path().join("notes");
    let db_path = tmp.path().join("test_theme.db");
    let backup_dir = tmp.path().join("backups");
    let assets_dir = tmp.path().join("assets");
    fs::create_dir_all(&notes_dir).unwrap();
    fs::create_dir_all(&assets_dir).unwrap();

    let config = ServerConfig {
        notes_dir: notes_dir.clone(),
        assets_dir,
        db_path,
        backup_dir,
        ..Default::default()
    };

    let server_handle = start_server_with_config(config).expect("Server should start");
    let port = server_handle.port();

    // 1. Initial theme query returns empty JSON object
    let res = ureq::get(&format!("http://127.0.0.1:{}/api/theme", port)).call().unwrap();
    assert_eq!(res.status(), 200);
    let theme_json: serde_json::Value = res.into_json().unwrap();
    assert!(theme_json.as_object().unwrap().is_empty());

    // 2. Set theme colors via ServerContext
    let mut colors = std::collections::HashMap::new();
    colors.insert("colorScheme".to_string(), "dark".to_string());
    colors.insert("primaryColor".to_string(), "#ffffff".to_string());
    colors.insert("secondaryColor".to_string(), "#80ffffff".to_string());
    colors.insert("highlightColor".to_string(), "#52b5ff".to_string());
    colors.insert("highlightBackgroundColor".to_string(), "#3352b5ff".to_string());
    colors.insert("primary".to_string(), "#52b5ff".to_string());

    server_handle.context().set_theme_colors(colors.clone());

    // 3. Query /api/theme again to verify updated theme colors
    let res2 = ureq::get(&format!("http://127.0.0.1:{}/api/theme", port)).call().unwrap();
    assert_eq!(res2.status(), 200);
    let theme_json2: serde_json::Value = res2.into_json().unwrap();
    assert_eq!(theme_json2["colorScheme"], "dark");
    assert_eq!(theme_json2["highlightColor"], "#52b5ff");
    assert_eq!(theme_json2["primaryColor"], "#ffffff");
    assert_eq!(theme_json2["secondaryColor"], "#80ffffff");
    assert_eq!(theme_json2["highlightBackgroundColor"], "#3352b5ff");

    // 4. Verify QtThemeColors::from_map compatibility
    let qt_theme = notesplusplus_core::html::qt_html::QtThemeColors::from_map(&colors);
    assert_eq!(qt_theme.highlight_color, "#52b5ff");
    assert_eq!(qt_theme.primary_color, "#ffffff");
    assert_eq!(qt_theme.highlight_background_color, "#3352b5ff");

    server_handle.stop();
}

#[test]
fn test_server_start_with_theme_config() {
    use std::ffi::CString;

    let tmp = tempdir().unwrap();
    let notes_dir = tmp.path().join("notes");
    let db_path = tmp.path().join("test_theme_start.db");
    let backup_dir = tmp.path().join("backups");
    fs::create_dir_all(&notes_dir).unwrap();

    let c_notes_dir = CString::new(notes_dir.to_str().unwrap()).unwrap();
    let c_db_path = CString::new(db_path.to_str().unwrap()).unwrap();
    let c_backup_dir = CString::new(backup_dir.to_str().unwrap()).unwrap();

    let config_json = json!({
        "theme": {
            "colorScheme": "dark",
            "highlightColor": "#ff5500",
            "primaryColor": "#ffffff",
            "secondaryColor": "#80ffffff",
            "highlightBackgroundColor": "#33ff5500"
        }
    });
    let c_config = CString::new(config_json.to_string()).unwrap();

    let handle = notesplusplus_core::ffi::notes_core_server_start(
        c_notes_dir.as_ptr(),
        c_db_path.as_ptr(),
        c_backup_dir.as_ptr(),
        0,
        c_config.as_ptr(),
    );
    assert!(!handle.is_null());

    let port = notesplusplus_core::ffi::notes_core_server_port(handle);
    assert!(port > 0);

    let res = ureq::get(&format!("http://127.0.0.1:{}/api/theme", port)).call().unwrap();
    assert_eq!(res.status(), 200);
    let theme_json: serde_json::Value = res.into_json().unwrap();
    assert_eq!(theme_json["colorScheme"], "dark");
    assert_eq!(theme_json["highlightColor"], "#ff5500");
    assert_eq!(theme_json["primaryColor"], "#ffffff");
    assert_eq!(theme_json["secondaryColor"], "#80ffffff");
    assert_eq!(theme_json["highlightBackgroundColor"], "#33ff5500");

    notesplusplus_core::ffi::notes_core_server_stop(handle);
}

#[test]
fn test_theme_assets_and_contrast_rules() {
    let tmp = tempdir().unwrap();
    let notes_dir = tmp.path().join("notes");
    let db_path = tmp.path().join("test_theme_assets.db");
    let backup_dir = tmp.path().join("backups");
    let assets_dir = tmp.path().join("assets");
    fs::create_dir_all(&notes_dir).unwrap();
    fs::create_dir_all(&assets_dir).unwrap();

    let server_handle = start_server_full(notes_dir.clone(), db_path, backup_dir, 18998, None, None).expect("Server should start");
    let port = server_handle.port();

    // 1. Check style.css defines data-theme, heading-color, top icons and view-mode styling
    let css_res = ureq::get(&format!("http://127.0.0.1:{}/style.css", port)).call().unwrap();
    assert_eq!(css_res.status(), 200);
    let css = css_res.into_string().unwrap();
    assert!(css.contains("[data-theme=\"light\"]"));
    assert!(css.contains("[data-theme=\"dark\"]"));
    assert!(css.contains("--heading-color"));
    assert!(css.contains(".notes-body h1, .notes-body h2"));
    assert!(css.contains("var(--heading-color"));
    assert!(css.contains(".nav-group .btn-icon"));
    assert!(css.contains(".view-mode-tabs button.active svg"));
    assert!(css.contains(".view-mode-tabs button.active .tab-label"));

    // 2. Check index.html has theme selector
    let html_res = ureq::get(&format!("http://127.0.0.1:{}/", port)).call().unwrap();
    assert_eq!(html_res.status(), 200);
    let html = html_res.into_string().unwrap();
    assert!(html.contains("themePreference"));
    assert!(html.contains("Auto (OS Ambiance)"));
    assert!(html.contains("System (Browser)"));

    // 3. Check theme store is served
    let theme_js_res = ureq::get(&format!("http://127.0.0.1:{}/stores/theme.js", port)).call().unwrap();
    assert_eq!(theme_js_res.status(), 200);
    let theme_js = theme_js_res.into_string().unwrap();
    assert!(theme_js.contains("useThemeStore"));
    assert!(theme_js.contains("notesplus_theme_preference"));

    server_handle.stop();
}

#[test]
fn test_events_sse_endpoint() {
    let tmp = tempdir().unwrap();
    let notes_dir = tmp.path().join("notes");
    let db_path = tmp.path().join("test_events.db");
    let backup_dir = tmp.path().join("backups");
    let assets_dir = tmp.path().join("assets");
    fs::create_dir_all(&notes_dir).unwrap();
    fs::create_dir_all(&assets_dir).unwrap();

    let server_handle = start_server_full(notes_dir, db_path, backup_dir, 18999, None, None).expect("Server should start");
    let port = server_handle.port();

    let sess = server_handle.context().session_store.create_session("admin", "code", 3600).unwrap();
    let session_cookie = format!("{}={}", notesplusplus_core::constants::SESSION_COOKIE_NAME, sess.id);

    let res = ureq::get(&format!("http://127.0.0.1:{}/api/events", port))
        .set("Cookie", &session_cookie)
        .call()
        .unwrap();
    assert_eq!(res.status(), 200);
    assert_eq!(res.header("Content-Type").unwrap(), "text/event-stream");

    let mut reader = res.into_reader();
    let mut buf = [0u8; 1024];
    use std::io::Read;
    let n = reader.read(&mut buf).unwrap();
    let body = String::from_utf8_lossy(&buf[..n]);
    assert!(body.contains("data:"));
    assert!(body.contains("connected"));
    assert!(body.contains("Notes Plus"));

    server_handle.stop();
}

#[test]
fn test_tree_api_and_adr_path_fetching() {
    let tmp = tempdir().unwrap();
    let notes_dir = tmp.path().join("notes");
    let adr_dir = notes_dir.join("Notes_Plus_Documentation").join("ADRs");
    let db_path = tmp.path().join("test_adr_tree.db");
    let backup_dir = tmp.path().join("backups");
    let assets_dir = tmp.path().join("assets");
    fs::create_dir_all(&adr_dir).unwrap();
    fs::create_dir_all(&assets_dir).unwrap();

    let server_handle = start_server_full(notes_dir.clone(), db_path, backup_dir, 19001, None, None).expect("Server should start");
    let port = server_handle.port();

    // Create a note inside ADRs directory
    let adr_note_content = "= ADR-001: Hybrid Core and GUI Architecture\n\n== Context\nThis document captures the architectural decisions.\n\n[source,rust]\n----\nfn init() {}\n----\n";
    let adr_file_path = adr_dir.join("001-hybrid-core-and-gui-architecture.adoc");
    fs::write(&adr_file_path, adr_note_content).unwrap();

    // Sync and index
    server_handle.context().repository.sync_all().unwrap();

    // Create authenticated session
    let sess = server_handle.context().session_store.create_session("admin", "code", 3600).unwrap();
    let session_cookie = format!("{}={}", notesplusplus_core::constants::SESSION_COOKIE_NAME, sess.id);

    // Test GET /api/tree returns structured preview blocks with rendered HTML
    let tree_res = ureq::get(&format!("http://127.0.0.1:{}/api/tree", port))
        .set("Cookie", &session_cookie)
        .call()
        .unwrap();
    assert_eq!(tree_res.status(), 200);
    let tree_json: serde_json::Value = tree_res.into_json().unwrap();
    assert!(tree_json.is_array());

    // Traverse tree nodes to find the ADR note
    fn find_page_in_tree<'a>(nodes: &'a [serde_json::Value], title_substr: &str) -> Option<&'a serde_json::Value> {
        for n in nodes {
            if let Some(pages) = n.get("pages").and_then(|p| p.as_array()) {
                for page in pages {
                    let title = page.get("name").or_else(|| page.get("title")).and_then(|t| t.as_str()).unwrap_or("");
                    if title.contains(title_substr) {
                        return Some(page);
                    }
                }
            }
            if let Some(children) = n.get("children").and_then(|c| c.as_array()) {
                if let Some(found) = find_page_in_tree(children, title_substr) {
                    return Some(found);
                }
            }
        }
        None
    }

    let adr_page = find_page_in_tree(tree_json.as_array().unwrap(), "Hybrid Core").expect("ADR note should be in tree");
    assert_eq!(adr_page["name"], "ADR-001: Hybrid Core and GUI Architecture");
    assert!(adr_page["color"].is_string());
    assert!(adr_page["id"].is_number());
    let preview_blocks = adr_page["preview_blocks"].as_array().expect("Preview blocks array");
    assert!(!preview_blocks.is_empty(), "Preview blocks must have content");
    assert!(preview_blocks[0]["html"].as_str().unwrap().contains("ADR-001"));
    assert!(preview_blocks[0]["text"].as_str().unwrap().contains("ADR-001"));

    // Test fetching ADR note via full path and filename
    let full_path = "Notes_Plus_Documentation/ADRs/001-hybrid-core-and-gui-architecture.adoc";
    let get_note_res = ureq::get(&format!("http://127.0.0.1:{}/api/notes/{}", port, full_path))
        .set("Cookie", &session_cookie)
        .call()
        .unwrap();
    assert_eq!(get_note_res.status(), 200);
    let raw_text = get_note_res.into_string().unwrap();
    assert!(raw_text.contains("ADR-001: Hybrid Core"));

    let get_page_res = ureq::get(&format!("http://127.0.0.1:{}/api/pages/{}", port, full_path))
        .set("Cookie", &session_cookie)
        .call()
        .unwrap();
    assert_eq!(get_page_res.status(), 200);
    let page_detail: serde_json::Value = get_page_res.into_json().unwrap();
    assert_eq!(page_detail["title"], "ADR-001: Hybrid Core and GUI Architecture");
    assert!(page_detail["html"].as_str().unwrap().contains("ADR-001"));

    server_handle.stop();
}

#[test]
fn test_contextual_navbar_and_blocks_mode_removal() {
    use notesplusplus_core::server::web_assets::{
        INDEX_HTML, STYLE_CSS, STORES_EDITOR_STORE_JS, STORES_LINK_STORE_JS, STORES_NOTES_JS,
    };

    // 1. Verify index.html contains contextual navbar bindings
    assert!(INDEX_HTML.contains("v-if=\"viewMode === 'gallery'\""));
    assert!(INDEX_HTML.contains("btn-new"));
    assert!(INDEX_HTML.contains("btn-back-gallery"));
    assert!(INDEX_HTML.contains("current-note-title"));
    assert!(INDEX_HTML.contains("v-if=\"viewMode !== 'gallery'\""));
    assert!(INDEX_HTML.contains("btn-save"));
    assert!(INDEX_HTML.contains("export-dropdown"));

    // 2. Verify Blocks (inplace) mode is removed from index.html
    assert!(!INDEX_HTML.contains("switchToInPlaceMode"));
    assert!(!INDEX_HTML.contains("inplace-container"));
    assert!(!INDEX_HTML.contains("inplace-editor-card"));
    assert!(!INDEX_HTML.contains("inPlaceBlocks"));

    // 3. Verify editorStore.js has in-place logic removed while preserving core helpers
    assert!(!STORES_EDITOR_STORE_JS.contains("inPlaceBlocks"));
    assert!(!STORES_EDITOR_STORE_JS.contains("switchToInPlaceMode"));
    assert!(!STORES_EDITOR_STORE_JS.contains("editBlock"));
    assert!(STORES_EDITOR_STORE_JS.contains("insertPrefix"));
    assert!(STORES_EDITOR_STORE_JS.contains("wrapSelection"));
    assert!(STORES_EDITOR_STORE_JS.contains("insertTab"));
    assert!(STORES_EDITOR_STORE_JS.contains("insertTableTemplate"));
    assert!(STORES_EDITOR_STORE_JS.contains("handlePreviewClick"));

    // 4. Verify linkStore.js and notes.js do not reference inplace
    assert!(!STORES_LINK_STORE_JS.contains("inplace"));
    assert!(!STORES_NOTES_JS.contains("inplace-rendered-card"));

    // 5. Verify style.css contains contextual navigation styles and no inplace blocks
    assert!(!STYLE_CSS.contains(".inplace-container"));
    assert!(!STYLE_CSS.contains(".inplace-editor-card"));
    assert!(STYLE_CSS.contains(".btn-back-gallery"));
    assert!(STYLE_CSS.contains(".current-note-title"));
    assert!(STYLE_CSS.contains(".btn-new"));
}

#[test]
fn test_api_search_content_and_payload() {
    let tmp = tempdir().unwrap();
    let notes_dir = tmp.path().join("notes");
    let db_path = tmp.path().join("test_search.db");
    let backup_dir = tmp.path().join("backups");
    let docs_dir = notes_dir.join("Guides");
    fs::create_dir_all(&docs_dir).unwrap();

    let server_handle = start_server_full(notes_dir.clone(), db_path, backup_dir, 19002, None, None).expect("Server should start");
    let port = server_handle.port();

    // Create notes with distinct content keywords
    let note1_content = "= SQLite Full Text Indexing\n\nWe utilize SQLite FTS5 for lightning fast full-text searching across all pages.\n";
    let note1_path = docs_dir.join("sqlite-fts5-guide.adoc");
    fs::write(&note1_path, note1_content).unwrap();

    let note2_content = "= Machine Learning Notebook\n\nDeep neural networks trained with stochastic gradient descent.\n";
    let note2_path = notes_dir.join("ml-notes.adoc");
    fs::write(&note2_path, note2_content).unwrap();

    // Sync and index into SQLite & FTS5
    server_handle.context().repository.sync_all().unwrap();

    // Authenticate session
    let sess = server_handle.context().session_store.create_session("admin", "code", 3600).unwrap();
    let session_cookie = format!("{}={}", notesplusplus_core::constants::SESSION_COOKIE_NAME, sess.id);

    // 1. Test searching content with ?q=
    let search_res = ureq::get(&format!("http://127.0.0.1:{}/api/search?q=lightning", port))
        .set("Cookie", &session_cookie)
        .call()
        .unwrap();
    assert_eq!(search_res.status(), 200);
    let items: serde_json::Value = search_res.into_json().unwrap();
    let arr = items.as_array().expect("Expected JSON array");
    assert_eq!(arr.len(), 1);
    let item = &arr[0];
    assert_eq!(item["title"], "SQLite Full Text Indexing");
    assert_eq!(item["name"], "SQLite Full Text Indexing");
    assert_eq!(item["filename"], "sqlite-fts5-guide.adoc");
    assert_eq!(item["group_path"], "Guides");
    assert_eq!(item["full_path"], "Guides/sqlite-fts5-guide.adoc");
    assert!(item["color"].is_string());
    assert!(item["id"].is_number());
    assert!(item["created_at"].is_string());
    assert!(item["updated_at"].is_string());
    assert!(item["snippet"].as_str().unwrap().contains("<b>lightning</b>"));

    // 2. Test searching content with ?search= (alias)
    let search_res2 = ureq::get(&format!("http://127.0.0.1:{}/api/search?search=neural", port))
        .set("Cookie", &session_cookie)
        .call()
        .unwrap();
    assert_eq!(search_res2.status(), 200);
    let items2: serde_json::Value = search_res2.into_json().unwrap();
    let arr2 = items2.as_array().expect("Expected JSON array");
    assert_eq!(arr2.len(), 1);
    let item2 = &arr2[0];
    assert_eq!(item2["title"], "Machine Learning Notebook");
    assert_eq!(item2["group_path"], "");
    assert_eq!(item2["full_path"], "ml-notes.adoc");
    assert!(item2["snippet"].as_str().unwrap().contains("<b>neural</b>"));

    // 3. Test searching nonexistent query returns empty array
    let search_res3 = ureq::get(&format!("http://127.0.0.1:{}/api/search?q=nonexistentterm12345", port))
        .set("Cookie", &session_cookie)
        .call()
        .unwrap();
    assert_eq!(search_res3.status(), 200);
    let items3: serde_json::Value = search_res3.into_json().unwrap();
    assert_eq!(items3.as_array().unwrap().len(), 0);

    server_handle.stop();
}

#[test]
fn test_rebuild_index_and_search_chronicles_wolpertinger() {
    let tmp = tempdir().unwrap();
    let notes_dir = tmp.path().join("notes");
    let db_path = tmp.path().join("test_rebuild_wolpertinger.db");
    let backup_dir = tmp.path().join("backups");
    fs::create_dir_all(&notes_dir).unwrap();

    let chronicles_content = "= The Dangerous & Thrilling Documentation Chronicles\n\n\
        During the opening university session, a script-happy warlock inadvertently released a legion of Wolpertingers!\n\
        Beware, it's a favorite of the Wolpertinger.\n";
    fs::write(notes_dir.join("chronicles.adoc"), chronicles_content).unwrap();

    let server_handle = start_server_full(notes_dir.clone(), db_path.clone(), backup_dir, 19003, None, None).expect("Server should start");
    let port = server_handle.port();

    // Authenticate session
    let sess = server_handle.context().session_store.create_session("admin", "code", 3600).unwrap();
    let session_cookie = format!("{}={}", notesplusplus_core::constants::SESSION_COOKIE_NAME, sess.id);

    // Perform rebuild_index as Settings > Display > Rebuild Index would do
    let conn = notesplusplus_core::db::open_db(&db_path).unwrap();
    let stats = notesplusplus_core::page::rebuild_index(&conn, &notes_dir).expect("rebuild_index should succeed");
    assert_eq!(stats.pages_indexed, 1);

    // Search via HTTP API for wolpertinger
    let search_res = ureq::get(&format!("http://127.0.0.1:{}/api/search?q=wolpertinger", port))
        .set("Cookie", &session_cookie)
        .call()
        .unwrap();
    assert_eq!(search_res.status(), 200);

    let items: serde_json::Value = search_res.into_json().unwrap();
    let arr = items.as_array().expect("Expected JSON array");
    assert_eq!(arr.len(), 1, "Should find chronicles.adoc for wolpertinger");
    assert_eq!(arr[0]["filename"], "chronicles.adoc");
    let snippet = arr[0]["snippet"].as_str().unwrap().to_lowercase();
    assert!(snippet.contains("wolpertinger"));

    server_handle.stop();
}

#[test]
fn test_note_color_selection_and_persistence() {
    let tmp = tempdir().unwrap();
    let notes_dir = tmp.path().join("notes");
    let db_path = tmp.path().join("test_note_colors.db");
    let backup_dir = tmp.path().join("backups");
    fs::create_dir_all(&notes_dir).unwrap();

    let server_handle = start_server_full(notes_dir.clone(), db_path.clone(), backup_dir, 19004, None, None).expect("Server should start");
    let port = server_handle.port();

    let sess = server_handle.context().session_store.create_session("admin", "code", 3600).unwrap();
    let session_cookie = format!("{}={}", notesplusplus_core::constants::SESSION_COOKIE_NAME, sess.id);

    // 1. Create a note with a chosen color
    let create_payload = json!({
        "title": "Design Specs",
        "color": "#e74c3c"
    });
    let create_res = ureq::post(&format!("http://127.0.0.1:{}/api/notes", port))
        .set("Cookie", &session_cookie)
        .set("Content-Type", "application/json")
        .send_json(create_payload)
        .unwrap();
    assert_eq!(create_res.status(), 200);
    let created_info: serde_json::Value = create_res.into_json().unwrap();
    assert_eq!(created_info["color"], "#e74c3c");
    assert_eq!(created_info["custom_color"], "#e74c3c");

    // 2. Query GET /api/notes/Design_Specs.adoc/color
    let get_color_res = ureq::get(&format!("http://127.0.0.1:{}/api/notes/Design_Specs.adoc/color", port))
        .set("Cookie", &session_cookie)
        .call()
        .unwrap();
    assert_eq!(get_color_res.status(), 200);
    let color_info: serde_json::Value = get_color_res.into_json().unwrap();
    assert_eq!(color_info["color"], "#e74c3c");
    assert_eq!(color_info["custom_color"], "#e74c3c");

    // 3. Update color via PUT /api/notes/Design_Specs.adoc/color to #00b894
    let update_color_res = ureq::put(&format!("http://127.0.0.1:{}/api/notes/Design_Specs.adoc/color", port))
        .set("Cookie", &session_cookie)
        .set("Content-Type", "application/json")
        .send_json(json!({ "color": "#00b894" }))
        .unwrap();
    assert_eq!(update_color_res.status(), 200);
    let updated_info: serde_json::Value = update_color_res.into_json().unwrap();
    assert_eq!(updated_info["color"], "#00b894");
    assert_eq!(updated_info["custom_color"], "#00b894");

    // 4. Verify /api/tree reflects the updated color
    let tree_res = ureq::get(&format!("http://127.0.0.1:{}/api/tree", port))
        .set("Cookie", &session_cookie)
        .call()
        .unwrap();
    assert_eq!(tree_res.status(), 200);
    let tree_json: serde_json::Value = tree_res.into_json().unwrap();
    let tree_arr = tree_json.as_array().expect("Expected tree array");
    let root_group = &tree_arr[0];
    let pages_arr = root_group["pages"].as_array().expect("Expected pages array");
    let found_page = pages_arr.iter().find(|p| p["title"] == "Design Specs").expect("Page should exist in tree");
    assert_eq!(found_page["color"], "#00b894");
    assert_eq!(found_page["custom_color"], "#00b894");

    // 5. Reset color to default (empty string)
    let reset_res = ureq::put(&format!("http://127.0.0.1:{}/api/notes/Design_Specs.adoc/color", port))
        .set("Cookie", &session_cookie)
        .set("Content-Type", "application/json")
        .send_json(json!({ "color": "" }))
        .unwrap();
    assert_eq!(reset_res.status(), 200);
    let reset_info: serde_json::Value = reset_res.into_json().unwrap();
    assert_eq!(reset_info["custom_color"], "");
    assert_eq!(reset_info["color"], notesplusplus_core::page::compute_note_color("Design Specs"));

    server_handle.stop();
}
