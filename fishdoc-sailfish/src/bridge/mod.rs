use qmetaobject::*;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use fishdoc_core::block::Block;
use fishdoc_core::db;
use fishdoc_core::page;

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

/// QML bridge exposing FishDoc functionality to the UI.
#[derive(QObject)]
pub struct FishdocBridge {
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
    web_server_running: qt_property!(bool; NOTIFY web_server_status_changed),
    web_server_url: qt_property!(String; NOTIFY web_server_status_changed),
    error_message: qt_property!(String; NOTIFY error_occurred),

    // Signals
    page_changed: qt_signal!(),
    search_results_changed: qt_signal!(),
    data_refreshed: qt_signal!(),
    loading_changed: qt_signal!(),
    drop_comments_changed: qt_signal!(),
    web_server_status_changed: qt_signal!(),
    error_occurred: qt_signal!(message: String),
    page_saved: qt_signal!(),
    html_exported: qt_signal!(path: String),

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
    load_main_page_data: qt_method!(fn(&mut self)),
    poll_results: qt_method!(fn(&mut self) -> bool),
    export_html: qt_method!(fn(&mut self, page_name: String) -> String),
    export_all_html: qt_method!(fn(&mut self) -> String),
    open_in_browser: qt_method!(fn(&mut self, page_name: String)),
    start_web_server: qt_method!(fn(&mut self) -> String),
    stop_web_server: qt_method!(fn(&mut self)),
    toggle_web_server: qt_method!(fn(&mut self) -> bool),

    // Internal state
    conn: Option<rusqlite::Connection>,
    notes_path: PathBuf,
    data_dir: PathBuf,
    current_blocks_data: Vec<Block>,
    journal_blocks_data: Vec<Block>,
    pending: Arc<Mutex<Option<PendingResult>>>,
    server_handle: Option<fishdoc_core::server::HttpServerHandle>,
}

impl Default for FishdocBridge {
    fn default() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        let data_dir = PathBuf::from(&home).join(".local").join("share").join("harbour-fishdoc");
        let notes_path = data_dir.join("notes");
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
            web_server_running: false,
            web_server_url: String::new(),
            error_message: String::new(),
            page_changed: Default::default(),
            search_results_changed: Default::default(),
            data_refreshed: Default::default(),
            loading_changed: Default::default(),
            drop_comments_changed: Default::default(),
            web_server_status_changed: Default::default(),
            error_occurred: Default::default(),
            page_saved: Default::default(),
            html_exported: Default::default(),
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
            load_main_page_data: Default::default(),
            poll_results: Default::default(),
            export_html: Default::default(),
            export_all_html: Default::default(),
            open_in_browser: Default::default(),
            start_web_server: Default::default(),
            stop_web_server: Default::default(),
            toggle_web_server: Default::default(),
            conn: None,
            notes_path,
            data_dir,
            current_blocks_data: Vec::new(),
            journal_blocks_data: Vec::new(),
            pending: Arc::new(Mutex::new(None)),
            server_handle: None,
        }
    }
}

impl FishdocBridge {
    fn ensure_init(&mut self) {
        if self.conn.is_some() {
            return;
        }

        let _ = std::fs::create_dir_all(&self.notes_path);
        let _ = std::fs::create_dir_all(self.data_dir.join("exports"));

        let db_path = self.data_dir.join("fishdoc.db");
        match rusqlite::Connection::open(&db_path) {
            Ok(conn) => {
                if let Err(e) = db::init_schema(&conn) {
                    self.error_message = format!("DB init error: {}", e);
                    self.error_occurred(self.error_message.clone());
                    return;
                }
                let _ = page::copy_examples(&conn, &self.notes_path, std::path::Path::new("/usr/share/harbour-fishdoc/examples"));
                let _ = page::sync_and_index_pages(&conn, &self.notes_path);
                self.conn = Some(conn);
            }
            Err(e) => {
                self.error_message = format!("DB open error: {}", e);
                self.error_occurred(self.error_message.clone());
            }
        }
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
