use qmetaobject::*;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::sync::mpsc;

use notesplusplus_core::block::Block;
use notesplusplus_core::db;
use notesplusplus_core::inline::InlineSpan;
use notesplusplus_core::page;

mod pages;
mod server_bridge;
mod journal_bridge;
mod search_bridge;
pub mod agent_bridge;
pub mod speech_bridge;

pub use agent_bridge::AgentBridge;
pub use speech_bridge::SpeechBridge;

/// Pending async result from load_page
pub(super) struct PendingResult {
    pub blocks: Option<Vec<Block>>,
    pub error: Option<String>,
}

/// Result of background main-page preview computation.
struct MainPageData {
    /// Pre-built JSON strings for each recent page (one JSON object per page)
    recent_page_jsons: Vec<String>,
    grouped_tree_json: String,
}

/// QML bridge exposing Notes++ functionality to the UI.
#[derive(QObject)]
pub struct NotesBridge {
    base: qt_base_class!(trait QObject),

    // Properties
    current_page_name: qt_property!(String; NOTIFY page_changed),
    current_page_group_path: qt_property!(String; NOTIFY current_page_group_path_changed),
    current_page_full_path: qt_property!(String; NOTIFY current_page_full_path_changed),
    current_page_file_path: qt_property!(String; NOTIFY current_page_file_path_changed),
    current_blocks: qt_property!(QVariantList; NOTIFY page_changed),
    is_journal_page: qt_property!(bool; NOTIFY page_changed),
    blocks_version: qt_property!(i32; NOTIFY page_changed),
    notes_dir: qt_property!(String; NOTIFY page_changed),
    search_query: qt_property!(String; NOTIFY search_results_changed),
    search_results: qt_property!(QVariantList; NOTIFY search_results_changed),
    search_loading: qt_property!(bool; NOTIFY loading_changed),
    current_search_filenames: Vec<String>,
    current_search_jsons: Vec<String>,
    recent_pages: qt_property!(QVariantList; NOTIFY data_refreshed),
    grouped_tree_json: qt_property!(QString; NOTIFY data_refreshed),
    group_display_depth: qt_property!(i32; NOTIFY group_depth_changed),
    recent_journal_lines: qt_property!(QVariantList; NOTIFY data_refreshed),
    journal_blocks: qt_property!(QVariantList; NOTIFY data_refreshed),
    is_loading: qt_property!(bool; NOTIFY loading_changed),
    drop_comments: qt_property!(bool; NOTIFY drop_comments_changed),
    reject_public_networks: qt_property!(bool; NOTIFY reject_public_networks_changed),
    bind_address: qt_property!(String; NOTIFY bind_address_changed),
    web_server_running: qt_property!(bool; NOTIFY web_server_status_changed),
    web_server_url: qt_property!(String; NOTIFY web_server_status_changed),
    error_message: qt_property!(String; NOTIFY error_occurred),
    initialized: qt_property!(bool; NOTIFY initialized_changed),
    auth_challenge_pending: qt_property!(bool; NOTIFY auth_challenge_changed),
    auth_challenge_id: qt_property!(String; NOTIFY auth_challenge_changed),
    auth_verification_code: qt_property!(String; NOTIFY auth_challenge_changed),

    // Signals
    page_changed: qt_signal!(),
    current_page_group_path_changed: qt_signal!(),
    current_page_full_path_changed: qt_signal!(),
    current_page_file_path_changed: qt_signal!(),
    search_results_changed: qt_signal!(),
    data_refreshed: qt_signal!(),
    group_depth_changed: qt_signal!(),
    loading_changed: qt_signal!(),
    drop_comments_changed: qt_signal!(),
    reject_public_networks_changed: qt_signal!(),
    bind_address_changed: qt_signal!(),
    web_server_status_changed: qt_signal!(),
    error_occurred: qt_signal!(message: String),
    initialized_changed: qt_signal!(),
    auth_challenge_changed: qt_signal!(),

    // Methods
    load_page: qt_method!(fn(&mut self, name: String)),
    save_block: qt_method!(fn(&mut self, index: i32, raw_text: String)),
    save_block_range: qt_method!(fn(&mut self, start_index: i32, count: i32, raw_text: String)),
    append_to_current_page: qt_method!(fn(&mut self, text: String, is_task: bool)),
    save_journal_block: qt_method!(fn(&mut self, index: i32, raw_text: String)),
    toggle_journal_checkbox: qt_method!(fn(&mut self, block_index: i32, item_path: String)),
    append_to_journal: qt_method!(fn(&mut self, text: String, is_task: bool)),
    get_page_source: qt_method!(fn(&mut self, name: String) -> String),
    save_page_source: qt_method!(fn(&mut self, name: String, content: String)),
    create_page: qt_method!(fn(&mut self, name: String)),
    delete_page: qt_method!(fn(&mut self, name: String)),
    create_group: qt_method!(fn(&mut self, parent_path: String, name: String) -> bool),
    rename_group: qt_method!(fn(&mut self, old_path: String, new_name: String) -> bool),
    delete_group: qt_method!(fn(&mut self, path: String, recursive: bool) -> bool),
    move_page_to_group: qt_method!(fn(&mut self, page_full_path: String, target_group: String) -> bool),
    set_group_display_depth: qt_method!(fn(&mut self, depth: i32)),
    toggle_group_collapsed: qt_method!(fn(&mut self, group_path: String) -> bool),
    get_groups_json: qt_method!(fn(&mut self) -> String),
    rebuild_index: qt_method!(fn(&mut self) -> String),
    do_search: qt_method!(fn(&mut self, query: String)),
    search: qt_method!(fn(&mut self, query: String)),
    poll_search: qt_method!(fn(&mut self) -> bool),
    poll_search_previews: qt_method!(fn(&mut self) -> bool),
    get_linkable_pages_json: qt_method!(fn(&mut self, query: String) -> String),
    insert_link_at_cursor: qt_method!(fn(&mut self, block_idx: i32, cursor_pos: i32, target: String)),
    navigate_to_page: qt_method!(fn(&mut self, name: String)),
    toggle_checkbox: qt_method!(fn(&mut self, block_index: i32, item_path: String)),
    set_drop_comments: qt_method!(fn(&mut self, drop: bool)),
    set_reject_public_networks: qt_method!(fn(&mut self, reject: bool)),
    set_bind_address: qt_method!(fn(&mut self, addr: String)),
    get_network_interfaces_json: qt_method!(fn(&mut self) -> String),
    load_main_page_data: qt_method!(fn(&mut self)),
    poll_main_page_data: qt_method!(fn(&mut self) -> bool),
    poll_results: qt_method!(fn(&mut self) -> bool),
    export_html: qt_method!(fn(&mut self, page_name: String) -> String),
    export_all_html: qt_method!(fn(&mut self) -> String),
    open_in_browser: qt_method!(fn(&mut self, page_name: String) -> String),
    get_server_urls_json: qt_method!(fn(&mut self) -> String),
    start_web_server: qt_method!(fn(&mut self) -> String),
    stop_web_server: qt_method!(fn(&mut self)),
    toggle_web_server: qt_method!(fn(&mut self) -> bool),
    configure_ai: qt_method!(fn(&mut self, provider: String, url: String, model: String, key: String, timeout: i32, auto_read: bool, auto_create: bool, require_edit: bool, allow_self_signed: bool, allow_fetch: bool)),
    install_tls_certificate: qt_method!(fn(&mut self, cert_pem_or_path: String, key_pem_or_path: String) -> String),
    reset_tls_certificate: qt_method!(fn(&mut self) -> String),
    is_custom_tls_certificate: qt_method!(fn(&mut self) -> bool),
    get_tls_certificate_info_json: qt_method!(fn(&mut self) -> String),
    set_theme: qt_method!(fn(&mut self, colors_json: String)),
    set_session_expiry_hours: qt_method!(fn(&mut self, hours: i32)),
    check_auth_challenge: qt_method!(fn(&mut self) -> bool),
    approve_auth_challenge: qt_method!(fn(&mut self, challenge_id: String)),
    deny_auth_challenge: qt_method!(fn(&mut self, challenge_id: String)),

    // Internal state
    conn: Option<rusqlite::Connection>,
    notes_path: PathBuf,
    data_dir: PathBuf,
    current_blocks_data: Vec<Block>,
    journal_blocks_data: Vec<Block>,
    pending: Arc<Mutex<Option<PendingResult>>>,
    search_result_slot: Arc<Mutex<Option<Result<Vec<search_bridge::SearchHit>, String>>>>,
    search_preview_slot: Arc<Mutex<Option<Vec<(String, String)>>>>,
    pending_main_page: Arc<Mutex<Option<MainPageData>>>,
    server_handle: Option<notesplusplus_core::server::HttpServerHandle>,
    llm_config: notesplusplus_core::agent::LlmConfig,
    permission_config: notesplusplus_core::agent::PermissionConfig,
    auth_config: notesplusplus_core::server::auth::AuthConfig,
    conn_receiver: Option<mpsc::Receiver<Result<rusqlite::Connection, String>>>,
    pending_theme_colors: std::collections::HashMap<String, String>,
    qt_theme: notesplusplus_core::html::qt_html::QtThemeColors,
}

impl Default for NotesBridge {
    fn default() -> Self {
        let paths = notesplusplus_core::paths::AppPaths::new();
        let data_dir = paths.data_dir;
        let notes_path = paths.notes_dir;
        let notes_dir_str = notes_path.to_string_lossy().to_string();

        Self {
            base: Default::default(),
            current_page_name: String::new(),
            current_page_group_path: String::new(),
            current_page_full_path: String::new(),
            current_page_file_path: String::new(),
            current_blocks: QVariantList::default(),
            is_journal_page: false,
            blocks_version: 0,
            notes_dir: notes_dir_str,
            search_query: String::new(),
            search_results: QVariantList::default(),
            recent_pages: QVariantList::default(),
            grouped_tree_json: QString::from("[]"),
            group_display_depth: 2,
            recent_journal_lines: QVariantList::default(),
            journal_blocks: QVariantList::default(),
            is_loading: false,
            drop_comments: true,
            reject_public_networks: true,
            bind_address: "0.0.0.0".to_string(),
            web_server_running: false,
            web_server_url: String::new(),
            error_message: String::new(),
            initialized: false,
            page_changed: Default::default(),
            current_page_group_path_changed: Default::default(),
            current_page_full_path_changed: Default::default(),
            current_page_file_path_changed: Default::default(),
            search_results_changed: Default::default(),
            data_refreshed: Default::default(),
            group_depth_changed: Default::default(),
            loading_changed: Default::default(),
            drop_comments_changed: Default::default(),
            reject_public_networks_changed: Default::default(),
            bind_address_changed: Default::default(),
            web_server_status_changed: Default::default(),
            error_occurred: Default::default(),
            initialized_changed: Default::default(),
            load_page: Default::default(),
            save_block: Default::default(),
            save_block_range: Default::default(),
            append_to_current_page: Default::default(),
            save_journal_block: Default::default(),
            toggle_journal_checkbox: Default::default(),
            append_to_journal: Default::default(),
            get_page_source: Default::default(),
            save_page_source: Default::default(),
            create_page: Default::default(),
            delete_page: Default::default(),
            create_group: Default::default(),
            rename_group: Default::default(),
            delete_group: Default::default(),
            move_page_to_group: Default::default(),
            set_group_display_depth: Default::default(),
            toggle_group_collapsed: Default::default(),
            get_groups_json: Default::default(),
            rebuild_index: Default::default(),
            do_search: Default::default(),
            search: Default::default(),
            poll_search: Default::default(),
            poll_search_previews: Default::default(),
            get_linkable_pages_json: Default::default(),
            insert_link_at_cursor: Default::default(),
            navigate_to_page: Default::default(),
            toggle_checkbox: Default::default(),
            set_drop_comments: Default::default(),
            set_reject_public_networks: Default::default(),
            set_bind_address: Default::default(),
            get_network_interfaces_json: Default::default(),
            load_main_page_data: Default::default(),
            poll_main_page_data: Default::default(),
            poll_results: Default::default(),
            export_html: Default::default(),
            export_all_html: Default::default(),
            open_in_browser: Default::default(),
            get_server_urls_json: Default::default(),
            start_web_server: Default::default(),
            stop_web_server: Default::default(),
            toggle_web_server: Default::default(),
            configure_ai: Default::default(),
            install_tls_certificate: Default::default(),
            reset_tls_certificate: Default::default(),
            is_custom_tls_certificate: Default::default(),
            get_tls_certificate_info_json: Default::default(),
            set_theme: Default::default(),
            set_session_expiry_hours: Default::default(),
            check_auth_challenge: Default::default(),
            approve_auth_challenge: Default::default(),
            deny_auth_challenge: Default::default(),
            auth_challenge_pending: false,
            auth_challenge_id: String::new(),
            auth_verification_code: String::new(),
            auth_challenge_changed: Default::default(),
            conn: None,
            notes_path,
            data_dir,
            current_blocks_data: Vec::new(),
            journal_blocks_data: Vec::new(),
            pending: Arc::new(Mutex::new(None)),
            search_result_slot: Arc::new(Mutex::new(None)),
            search_preview_slot: Arc::new(Mutex::new(None)),
            pending_main_page: Arc::new(Mutex::new(None)),
            search_loading: false,
            current_search_filenames: Vec::new(),
            current_search_jsons: Vec::new(),
            server_handle: None,
            llm_config: notesplusplus_core::agent::LlmConfig::default(),
            permission_config: notesplusplus_core::agent::PermissionConfig::default(),
            auth_config: notesplusplus_core::server::auth::AuthConfig::default(),
            conn_receiver: None,
            pending_theme_colors: std::collections::HashMap::new(),
            qt_theme: notesplusplus_core::html::qt_html::QtThemeColors::default(),
        }
    }
}

impl NotesBridge {
    /// Non-blocking init: spawns a background thread for DB setup if not already started.
    /// Call `poll_init()` to check if the connection is ready.
    fn ensure_init(&mut self) {
        if self.conn.is_some() || self.conn_receiver.is_some() {
            return;
        }

        let (tx, rx) = mpsc::channel();
        self.conn_receiver = Some(rx);

        let notes_path = self.notes_path.clone();
        let data_dir = self.data_dir.clone();

        std::thread::spawn(move || {
            let t0 = std::time::Instant::now();
            let _ = std::fs::create_dir_all(&notes_path);
            let _ = std::fs::create_dir_all(data_dir.join("exports"));
            eprintln!("[debug] dirs created in {:?}", t0.elapsed());

            let db_path = data_dir.join(notesplusplus_core::constants::DB_FILENAME);
            match rusqlite::Connection::open(&db_path) {
                Ok(conn) => {
                    let t = std::time::Instant::now();
                    if let Err(e) = db::init_schema(&conn) {
                        let _ = tx.send(Err(format!("DB init error: {}", e)));
                        return;
                    }
                    eprintln!("[debug] init_schema in {:?}", t.elapsed());

                    let t = std::time::Instant::now();
                    let _ = page::copy_examples(&conn, &notes_path, std::path::Path::new("/usr/share/harbour-notesplusplus/examples"), "");
                    eprintln!("[debug] copy_examples in {:?}", t.elapsed());

                    let t = std::time::Instant::now();
                    let _ = page::sync_and_index_pages(&conn, &notes_path);
                    eprintln!("[debug] sync_and_index_pages in {:?}", t.elapsed());

                    eprintln!("[debug] background init total: {:?}", t0.elapsed());
                    let _ = tx.send(Ok(conn));
                }
                Err(e) => {
                    let _ = tx.send(Err(format!("DB open error: {}", e)));
                }
            }
        });
    }

    /// Poll for background init completion. Returns true if connection is ready.
    fn poll_init(&mut self) -> bool {
        if self.conn.is_some() {
            return true;
        }
        if let Some(rx) = self.conn_receiver.as_ref() {
            match rx.try_recv() {
                Ok(Ok(conn)) => {
                    self.conn = Some(conn);
                    self.conn_receiver = None;
                    self.initialized = true;
                    self.initialized_changed();
                    return true;
                }
                Ok(Err(e)) => {
                    self.report_error(e);
                    self.conn_receiver = None;
                    return false;
                }
                Err(mpsc::TryRecvError::Empty) => return false,
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.conn_receiver = None;
                    return false;
                }
            }
        }
        false
    }

    /// Ensures the DB connection is ready, blocking briefly if background init is still running.
    fn ensure_init_blocking(&mut self) -> bool {
        if self.conn.is_some() {
            return true;
        }
        self.ensure_init();
        if let Some(rx) = self.conn_receiver.take() {
            match rx.recv() {
                Ok(Ok(conn)) => {
                    self.conn = Some(conn);
                    self.initialized = true;
                    self.initialized_changed();
                    return true;
                }
                Ok(Err(e)) => {
                    self.report_error(e);
                }
                Err(e) => {
                    self.report_error(format!("DB init channel disconnected: {}", e));
                }
            }
        }
        self.conn.is_some()
    }

    fn conn(&self) -> Option<&rusqlite::Connection> {
        self.conn.as_ref()
    }

    pub(super) fn blocks_to_qvariantlist(blocks: &[Block]) -> QVariantList {
        let mut headings_vec = Vec::new();
        for (idx, block) in blocks.iter().enumerate() {
            if let Block::Heading { level, spans, .. } = block {
                if *level >= 1 && *level <= 5 {
                    let text = spans.iter().map(|s| s.plain_text()).collect::<Vec<_>>().join("");
                    let mut h_map = serde_json::Map::new();
                    h_map.insert("level".into(), serde_json::Value::Number((*level).into()));
                    h_map.insert("text".into(), serde_json::Value::String(text));
                    h_map.insert("index".into(), serde_json::Value::Number(idx.into()));
                    headings_vec.push(serde_json::Value::Object(h_map));
                }
            }
        }

        let mut list = QVariantList::default();
        for block in blocks {
            let mut json = block.to_qvariant_map();
            if let Block::Toc { .. } = block {
                if let serde_json::Value::Object(ref mut map) = json {
                    map.insert("headings".into(), serde_json::Value::Array(headings_vec.clone()));
                }
            }
            let qv = QString::from(serde_json::to_string(&json).unwrap_or_default());
            list.push(qv.into());
        }
        list
    }

    /// Like `blocks_to_qvariantlist` but adds a pre-rendered `"html"` field to each block
    /// that has a Rust-side HTML representation. Blocks without HTML (Code, Image, Toc, etc.)
    /// are passed through unchanged — QML delegates fall back to JS rendering for those.
    /// Also appends a synthetic "footnotes" block if any footnote spans are found.
    pub(super) fn blocks_to_qvariantlist_with_html(
        blocks: &[Block],
        theme: &notesplusplus_core::html::qt_html::QtThemeColors,
        options: &notesplusplus_core::html::qt_html::QtRenderOptions,
    ) -> QVariantList {
        let mut headings_vec = Vec::new();
        for (idx, block) in blocks.iter().enumerate() {
            if let Block::Heading { level, spans, .. } = block {
                if *level >= 1 && *level <= 5 {
                    let text = spans.iter().map(|s| s.plain_text()).collect::<Vec<_>>().join("");
                    let mut h_map = serde_json::Map::new();
                    h_map.insert("level".into(), serde_json::Value::Number((*level).into()));
                    h_map.insert("text".into(), serde_json::Value::String(text));
                    h_map.insert("index".into(), serde_json::Value::Number(idx.into()));
                    headings_vec.push(serde_json::Value::Object(h_map));
                }
            }
        }

        let mut list = QVariantList::default();
        for (idx, block) in blocks.iter().enumerate() {
            let mut json = block.to_qvariant_map();
            if let Block::Toc { .. } = block {
                if let serde_json::Value::Object(ref mut map) = json {
                    map.insert("headings".into(), serde_json::Value::Array(headings_vec.clone()));
                }
            }
            // Add pre-rendered HTML for blocks that the Rust renderer handles
            match block {
                Block::Heading { .. } | Block::Paragraph { .. } |
                Block::OrderedListItem { .. } | Block::UnorderedListItem { .. } |
                Block::DescriptionListItem { .. } | Block::CalloutListItem { .. } |
                Block::CodeBlock { .. } | Block::LiteralBlock { .. } |
                Block::Blockquote { .. } | Block::Verse { .. } |
                Block::Admonition { .. } | Block::Sidebar { .. } |
                Block::Example { .. } | Block::Open { .. } |
                Block::Table { .. } | Block::Image { .. } => {
                    let html = notesplusplus_core::html::qt_html::render_qt_block(block, idx, theme, options);
                    if let serde_json::Value::Object(ref mut map) = json {
                        map.insert("html".into(), serde_json::Value::String(html));
                    }
                }
                _ => {}
            }
            let qv = QString::from(serde_json::to_string(&json).unwrap_or_default());
            list.push(qv.into());
        }

        // Collect footnotes from all blocks and append a synthetic footnotes block
        let mut footnotes: Vec<(Option<String>, String)> = Vec::new();
        let mut seen_ids: Vec<String> = Vec::new();
        collect_footnotes(blocks, &mut footnotes, &mut seen_ids);
        if !footnotes.is_empty() {
            let mut fn_html = String::from("<hr/><p style='margin:4px 8px;font-weight:bold;color:__LINK_COLOR__;'>Footnotes</p>");
            for (i, (id, text)) in footnotes.iter().enumerate() {
                let num_label = (i + 1).to_string();
                let label = id.as_deref().unwrap_or(&num_label);
                let content = if text.is_empty() { label } else { text.as_str() };
                use std::fmt::Write;
                write!(fn_html, "<p style='margin:2px 8px;'>[{}] {}</p>",
                    notesplusplus_core::html::qt_html::escape_html_for_footnote(label),
                    notesplusplus_core::html::qt_html::escape_html_for_footnote(content)
                ).ok();
            }
            let mut fn_json = serde_json::Map::new();
            fn_json.insert("type".into(), serde_json::Value::String("footnotes".into()));
            fn_json.insert("html".into(), serde_json::Value::String(fn_html));
            let qv = QString::from(serde_json::to_string(&fn_json).unwrap_or_default());
            list.push(qv.into());
        }

        list
    }
}

fn collect_footnotes(blocks: &[Block], footnotes: &mut Vec<(Option<String>, String)>, seen_ids: &mut Vec<String>) {
    for block in blocks {
        match block {
            Block::Heading { spans, .. } | Block::Paragraph { spans, .. } => {
                collect_footnotes_from_spans(spans, footnotes, seen_ids);
            }
            Block::OrderedListItem { children, .. } | Block::UnorderedListItem { children, .. } |
            Block::DescriptionListItem { children, .. } | Block::CalloutListItem { children, .. } |
            Block::Blockquote { children, .. } | Block::Admonition { children, .. } |
            Block::Sidebar { children, .. } | Block::Example { children, .. } |
            Block::Open { children, .. } => {
                collect_footnotes(children, footnotes, seen_ids);
            }
            Block::Verse { spans, .. } => {
                for line_spans in spans {
                    collect_footnotes_from_spans(line_spans, footnotes, seen_ids);
                }
            }
            Block::Table { rows, .. } => {
                for row in rows {
                    for cell in row {
                        collect_footnotes(&cell.blocks, footnotes, seen_ids);
                    }
                }
            }
            _ => {}
        }
    }
}

fn collect_footnotes_from_spans(spans: &[InlineSpan], footnotes: &mut Vec<(Option<String>, String)>, seen_ids: &mut Vec<String>) {
    for span in spans {
        match span {
            InlineSpan::Footnote { id, text } => {
                let key = id.clone().unwrap_or_default();
                if key.is_empty() || !seen_ids.contains(&key) {
                    if !key.is_empty() {
                        seen_ids.push(key);
                    }
                    footnotes.push((id.clone(), text.clone()));
                }
            }
            InlineSpan::Bold(inner) | InlineSpan::Italic(inner) | InlineSpan::Monospace(inner) |
            InlineSpan::Superscript(inner) | InlineSpan::Subscript(inner) | InlineSpan::Mark(inner) => {
                collect_footnotes_from_spans(inner, footnotes, seen_ids);
            }
            _ => {}
        }
    }
}
