use qmetaobject::*;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::sync::mpsc;

use notesplusplus_core::block::Block;
use notesplusplus_core::db;
use notesplusplus_core::page;

mod pages;
mod journal_bridge;
mod search_bridge;
pub mod agent_bridge;

pub use agent_bridge::AgentBridge;

/// Pending async result from load_page
pub(super) struct PendingResult {
    pub blocks: Option<Vec<Block>>,
    pub error: Option<String>,
}

/// QML bridge exposing Notes++ functionality to the UI.
#[derive(QObject)]
pub struct NotesBridge {
    base: qt_base_class!(trait QObject),

    // Properties
    current_page_name: qt_property!(String; NOTIFY page_changed),
    current_blocks: qt_property!(QVariantList; NOTIFY page_changed),
    is_journal_page: qt_property!(bool; NOTIFY page_changed),
    blocks_version: qt_property!(i32; NOTIFY page_changed),
    notes_dir: qt_property!(String; NOTIFY page_changed),
    search_query: qt_property!(String; NOTIFY search_results_changed),
    search_results: qt_property!(QVariantList; NOTIFY search_results_changed),
    recent_pages: qt_property!(QVariantList; NOTIFY data_refreshed),
    recent_journal_lines: qt_property!(QVariantList; NOTIFY data_refreshed),
    journal_blocks: qt_property!(QVariantList; NOTIFY data_refreshed),
    is_loading: qt_property!(bool; NOTIFY loading_changed),
    drop_comments: qt_property!(bool; NOTIFY drop_comments_changed),
    reject_public_networks: qt_property!(bool; NOTIFY reject_public_networks_changed),
    web_server_running: qt_property!(bool; NOTIFY web_server_status_changed),
    web_server_url: qt_property!(String; NOTIFY web_server_status_changed),
    error_message: qt_property!(String; NOTIFY error_occurred),
    initialized: qt_property!(bool; NOTIFY initialized_changed),

    // Signals
    page_changed: qt_signal!(),
    search_results_changed: qt_signal!(),
    data_refreshed: qt_signal!(),
    loading_changed: qt_signal!(),
    drop_comments_changed: qt_signal!(),
    reject_public_networks_changed: qt_signal!(),
    web_server_status_changed: qt_signal!(),
    error_occurred: qt_signal!(message: String),
    page_saved: qt_signal!(),
    html_exported: qt_signal!(path: String),
    initialized_changed: qt_signal!(),

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
    do_search: qt_method!(fn(&mut self, query: String)),
    search: qt_method!(fn(&mut self, query: String)),
    get_linkable_pages_json: qt_method!(fn(&mut self, query: String) -> String),
    insert_link_at_cursor: qt_method!(fn(&mut self, block_idx: i32, cursor_pos: i32, target: String)),
    navigate_to_page: qt_method!(fn(&mut self, name: String)),
    toggle_checkbox: qt_method!(fn(&mut self, block_index: i32, item_path: String)),
    set_drop_comments: qt_method!(fn(&mut self, drop: bool)),
    set_reject_public_networks: qt_method!(fn(&mut self, reject: bool)),
    load_main_page_data: qt_method!(fn(&mut self)),
    poll_results: qt_method!(fn(&mut self) -> bool),
    export_html: qt_method!(fn(&mut self, page_name: String) -> String),
    export_all_html: qt_method!(fn(&mut self) -> String),
    open_in_browser: qt_method!(fn(&mut self, page_name: String)),
    start_web_server: qt_method!(fn(&mut self) -> String),
    stop_web_server: qt_method!(fn(&mut self)),
    toggle_web_server: qt_method!(fn(&mut self) -> bool),
    configure_ai: qt_method!(fn(&mut self, provider: String, url: String, model: String, key: String, timeout: i32, auto_read: bool, auto_create: bool, require_edit: bool, allow_self_signed: bool)),
    install_tls_certificate: qt_method!(fn(&mut self, cert_pem_or_path: String, key_pem_or_path: String) -> String),
    reset_tls_certificate: qt_method!(fn(&mut self) -> String),
    is_custom_tls_certificate: qt_method!(fn(&mut self) -> bool),
    get_tls_certificate_info_json: qt_method!(fn(&mut self) -> String),
    configure_auth: qt_method!(fn(&mut self, enabled: bool, basic_enabled: bool, username: String, password: String, oauth_enabled: bool, provider_name: String, issuer_url: String, client_id: String, client_secret: String, allowed_emails: String, allow_self_signed: bool)),
    get_auth_info_json: qt_method!(fn(&mut self) -> String),

    // Internal state
    conn: Option<rusqlite::Connection>,
    notes_path: PathBuf,
    data_dir: PathBuf,
    current_blocks_data: Vec<Block>,
    journal_blocks_data: Vec<Block>,
    pending: Arc<Mutex<Option<PendingResult>>>,
    server_handle: Option<notesplusplus_core::server::HttpServerHandle>,
    llm_config: notesplusplus_core::agent::LlmConfig,
    permission_config: notesplusplus_core::agent::PermissionConfig,
    auth_config: notesplusplus_core::server::auth::AuthConfig,
    conn_receiver: Option<mpsc::Receiver<Result<rusqlite::Connection, String>>>,
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
            current_blocks: QVariantList::default(),
            is_journal_page: false,
            blocks_version: 0,
            notes_dir: notes_dir_str,
            search_query: String::new(),
            search_results: QVariantList::default(),
            recent_pages: QVariantList::default(),
            recent_journal_lines: QVariantList::default(),
            journal_blocks: QVariantList::default(),
            is_loading: false,
            drop_comments: true,
            reject_public_networks: true,
            web_server_running: false,
            web_server_url: String::new(),
            error_message: String::new(),
            initialized: false,
            page_changed: Default::default(),
            search_results_changed: Default::default(),
            data_refreshed: Default::default(),
            loading_changed: Default::default(),
            drop_comments_changed: Default::default(),
            reject_public_networks_changed: Default::default(),
            web_server_status_changed: Default::default(),
            error_occurred: Default::default(),
            page_saved: Default::default(),
            html_exported: Default::default(),
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
            do_search: Default::default(),
            search: Default::default(),
            get_linkable_pages_json: Default::default(),
            insert_link_at_cursor: Default::default(),
            navigate_to_page: Default::default(),
            toggle_checkbox: Default::default(),
            set_drop_comments: Default::default(),
            set_reject_public_networks: Default::default(),
            load_main_page_data: Default::default(),
            poll_results: Default::default(),
            export_html: Default::default(),
            export_all_html: Default::default(),
            open_in_browser: Default::default(),
            start_web_server: Default::default(),
            stop_web_server: Default::default(),
            toggle_web_server: Default::default(),
            configure_ai: Default::default(),
            install_tls_certificate: Default::default(),
            reset_tls_certificate: Default::default(),
            is_custom_tls_certificate: Default::default(),
            get_tls_certificate_info_json: Default::default(),
            configure_auth: Default::default(),
            get_auth_info_json: Default::default(),
            conn: None,
            notes_path,
            data_dir,
            current_blocks_data: Vec::new(),
            journal_blocks_data: Vec::new(),
            pending: Arc::new(Mutex::new(None)),
            server_handle: None,
            llm_config: notesplusplus_core::agent::LlmConfig::default(),
            permission_config: notesplusplus_core::agent::PermissionConfig::default(),
            auth_config: notesplusplus_core::server::auth::AuthConfig::default(),
            conn_receiver: None,
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
            eprintln!("[startup] dirs created in {:?}", t0.elapsed());

            let db_path = data_dir.join(notesplusplus_core::constants::DB_FILENAME);
            match rusqlite::Connection::open(&db_path) {
                Ok(conn) => {
                    let t = std::time::Instant::now();
                    if let Err(e) = db::init_schema(&conn) {
                        let _ = tx.send(Err(format!("DB init error: {}", e)));
                        return;
                    }
                    eprintln!("[startup] init_schema in {:?}", t.elapsed());

                    let t = std::time::Instant::now();
                    let _ = page::copy_examples(&conn, &notes_path, std::path::Path::new("/usr/share/harbour-notesplusplus/examples"));
                    eprintln!("[startup] copy_examples in {:?}", t.elapsed());

                    let t = std::time::Instant::now();
                    let _ = page::sync_and_index_pages(&conn, &notes_path);
                    eprintln!("[startup] sync_and_index_pages in {:?}", t.elapsed());

                    eprintln!("[startup] background init total: {:?}", t0.elapsed());
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
                    self.error_message = e;
                    self.error_occurred(self.error_message.clone());
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
}
