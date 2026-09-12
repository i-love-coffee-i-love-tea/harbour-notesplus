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
    let notes_subdir = notes_dir.join("notes");
    let db_path = tmp.path().join("test.db");
    let backup_dir = tmp.path().join("backups");
    fs::create_dir_all(&notes_dir).unwrap();
    fs::create_dir_all(&notes_subdir).unwrap();

    fs::write(notes_subdir.join("welcome.adoc"), "= Welcome\nTest content for server.").unwrap();

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
    assert!(root_body.contains("Notes++"));

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
    let updated_file = fs::read_to_string(notes_subdir.join("welcome.adoc")).unwrap();
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
    assert!(notes_subdir.join("New_Doc.adoc").exists());

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
    fs::write(notes_subdir.join("welcome.adoc"), "= Tasks\n* [ ] Task 1\n* [x] Task 2").unwrap();
    let toggle_res = ureq::post(&format!("http://127.0.0.1:{}/api/notes/welcome.adoc/toggle", port))
        .set("Cookie", &session_cookie)
        .send_json(json!({
            "item_index": 0,
            "checked": true
        }))
        .unwrap();
    assert_eq!(toggle_res.status(), 200);
    let toggled_file = fs::read_to_string(notes_subdir.join("welcome.adoc")).unwrap();
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
    assert!(!notes_subdir.join("New_Doc.adoc").exists());

    // Stop server
    server_handle.stop();
}

#[test]
fn test_server_phone_auth_challenge_flow_and_multi_request_superseding() {
    let tmp = tempdir().unwrap();
    let notes_dir = tmp.path().join("notes");
    let notes_subdir = notes_dir.join("notes");
    let db_path = tmp.path().join("test_phone_auth.db");
    let backup_dir = tmp.path().join("backups");
    fs::create_dir_all(&notes_dir).unwrap();
    fs::create_dir_all(&notes_subdir).unwrap();

    fs::write(notes_subdir.join("protected.adoc"), "= Protected\nContent only for authenticated users.").unwrap();

    let auth_cfg = auth::AuthConfig::default();

    let config = ServerConfig {
        notes_subdir: notes_dir.join("notes"),
        notes_dir,
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
        notes_subdir: notes_dir.join("notes"),
        notes_dir,
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
    use notesplusplus_core::server::web_assets::{INDEX_HTML, APP_JS, STYLE_CSS};

    // 1. Verify index.html contains Link buttons and Link Dialog modal overlay
    assert!(INDEX_HTML.contains("openLinkDialog"));
    assert!(INDEX_HTML.contains("link-modal-overlay"));
    assert!(INDEX_HTML.contains("link-search-box"));
    assert!(INDEX_HTML.contains("link-pages-list"));
    assert!(INDEX_HTML.contains("linkDisplayText"));
    assert!(INDEX_HTML.contains("formattedLinkPreview"));
    assert!(INDEX_HTML.contains("confirmLinkInsert"));

    // 2. Verify app.js contains link dialog state, computeds, and shortcut handlers
    assert!(APP_JS.contains("openLinkModal"));
    assert!(APP_JS.contains("linkSearchQuery"));
    assert!(APP_JS.contains("filteredLinkPages"));
    assert!(APP_JS.contains("formattedLinkPreview"));
    assert!(APP_JS.contains("openLinkDialog"));
    assert!(APP_JS.contains("confirmLinkInsert"));
    assert!(APP_JS.contains("handleLinkKeydown"));
    assert!(APP_JS.contains("selectLinkTarget"));
    assert!(APP_JS.contains("isExternalUrl"));
    assert!(APP_JS.contains("computedCustomFilename"));

    // 3. Verify style.css contains link modal classes
    assert!(STYLE_CSS.contains(".link-modal-card"));
    assert!(STYLE_CSS.contains(".link-search-box"));
    assert!(STYLE_CSS.contains(".link-pages-list"));
    assert!(STYLE_CSS.contains(".link-page-item"));
    assert!(STYLE_CSS.contains(".link-preview-container"));
}

#[test]
fn test_ai_model_selection_web_assets() {
    use notesplusplus_core::server::web_assets::{INDEX_HTML, APP_JS, STYLE_CSS, COMPOSABLE_USE_AI_ASSISTANT_JS};

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

    // 2. Verify app.js imports and re-exports AI composable
    assert!(APP_JS.contains("useAiAssistant"));
    assert!(APP_JS.contains("availableModels"));
    assert!(APP_JS.contains("fetchAvailableModels"));
    assert!(APP_JS.contains("onModelSelect"));
    assert!(APP_JS.contains("isCurrentModelInList"));

    // Verify AI composable contains provider, model fetching, system prompt state & methods
    assert!(COMPOSABLE_USE_AI_ASSISTANT_JS.contains("/api/ai/models"));
    assert!(COMPOSABLE_USE_AI_ASSISTANT_JS.contains("aiConfig.value.system_prompt"));
    assert!(COMPOSABLE_USE_AI_ASSISTANT_JS.contains("aiConfig.value.provider"));

    // Verify composable does not expose or send server endpoints, tokens, or self-signed cert flags
    assert!(!COMPOSABLE_USE_AI_ASSISTANT_JS.contains("aiConfig.value.endpoint"));
    assert!(!COMPOSABLE_USE_AI_ASSISTANT_JS.contains("aiConfig.value.apiKey"));
    assert!(!COMPOSABLE_USE_AI_ASSISTANT_JS.contains("allow_self_signed"));

    // 3. Verify style.css contains styling for the model select
    assert!(STYLE_CSS.contains(".ai-model-select"));
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
    let notes_subdir = notes_dir.join("notes");
    let db_path = tmp.path().join("test_security.db");
    let backup_dir = tmp.path().join("backups");
    fs::create_dir_all(&notes_dir).unwrap();
    fs::create_dir_all(&notes_subdir).unwrap();

    // Put a .adoc file in the notes subdirectory (where notes live)
    fs::write(notes_subdir.join("secret.adoc"), "= Secret\nConfidential content.").unwrap();

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
    assert!(body.contains("Notes++"), "unauthenticated /raw/ should serve login page, not note content");

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
    assert!(page_body.contains("Notes++"), "unauthenticated /page/ should serve login page");

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
    assert!(logout_cookie_hdr.contains("Max-Age=0") || logout_cookie_hdr.contains("notesplusplus_session="));

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
    use notesplusplus_core::server::web_assets::{INDEX_HTML, APP_JS, STYLE_CSS};

    // 1. Verify index.html contains logout button and action bindings
    assert!(INDEX_HTML.contains("@click=\"logout\""));
    assert!(INDEX_HTML.contains("btn-logout"));
    assert!(INDEX_HTML.contains("Logout"));

    // 2. Verify app.js imports auth composable and exports logout handler
    assert!(APP_JS.contains("useAuth"));
    assert!(APP_JS.contains("logout,"));

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
    use notesplusplus_core::server::web_assets::{INDEX_HTML, APP_JS, STYLE_CSS};

    // 1. Verify index.html contains session chip and remaining text
    assert!(INDEX_HTML.contains("session-chip"));
    assert!(INDEX_HTML.contains("sessionRemainingText"));
    assert!(INDEX_HTML.contains("session-time"));

    // 2. Verify app.js imports auth composable and exports session timer state
    assert!(APP_JS.contains("useAuth"));
    assert!(APP_JS.contains("sessionRemainingText"));

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
    use notesplusplus_core::server::web_assets::{INDEX_HTML, APP_JS, STYLE_CSS};

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

    // 2. Verify app.js defines handlers, state, and API routes
    assert!(APP_JS.contains("importUrl"));
    assert!(APP_JS.contains("showUrlInput"));
    assert!(APP_JS.contains("useImport"));
    assert!(APP_JS.contains("isFetchingUrl"));
    assert!(APP_JS.contains("fetchUrlContent"));
    assert!(APP_JS.contains("onFileSelect"));
    assert!(APP_JS.contains("onFileDrop"));
    assert!(APP_JS.contains("loadServerFile"));
    assert!(APP_JS.contains("pasteClipboard"));
    assert!(APP_JS.contains("fetchNotesList"));
    assert!(!APP_JS.contains("loadNotesList"));

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
    let notes_subdir = notes_dir.join("notes");
    let db_path = tmp.path().join("test_import_endpoints.db");
    let backup_dir = tmp.path().join("backups");
    fs::create_dir_all(&notes_subdir).unwrap();

    fs::write(notes_subdir.join("imported-sample.adoc"), "= Sample Imported Note\nThis is a test note for import.").unwrap();

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
    let file_path = notes_subdir.join("imported-sample.adoc").to_str().unwrap().to_string();
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

    let composables = [
        "composables/utils.js", "composables/useAuth.js", "composables/useHealthCheck.js",
        "composables/usePresentation.js", "composables/useLinkModal.js",
        "composables/useAiAssistant.js", "composables/useImport.js",
    ];

    for path in &composables {
        let res = ureq::get(&format!("http://127.0.0.1:{}/{}", port, path)).call().unwrap();
        assert_eq!(res.status(), 200, "Expected 200 for {}", path);
        let body = res.into_string().unwrap();
        assert!(body.len() > 10, "Expected non-trivial content for {}", path);
    }

    let app_js = ureq::get(&format!("http://127.0.0.1:{}/app.js", port)).call().unwrap().into_string().unwrap();
    assert!(app_js.contains("from '/composables/utils.js'") || app_js.contains("from './composables/utils.js'"), "app.js should import from composables/utils.js");

    server_handle.stop();
}
