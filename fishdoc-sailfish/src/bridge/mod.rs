use qmetaobject::*;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use fishdoc_core::block::Block;
use fishdoc_core::db;
use fishdoc_core::journal;
use fishdoc_core::page;
use fishdoc_core::parser;
use fishdoc_core::search as search_mod;

mod pages;
mod journal_bridge;
mod search_bridge;

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
    search_query: qt_property!(String; NOTIFY search_results_changed),
    search_results: qt_property!(QVariantList; NOTIFY search_results_changed),
    recent_pages: qt_property!(QVariantList; NOTIFY data_refreshed),
    recent_journal_lines: qt_property!(QVariantList; NOTIFY data_refreshed),
    is_loading: qt_property!(bool; NOTIFY loading_changed),
    error_message: qt_property!(String; NOTIFY error_occurred),

    // Signals
    page_changed: qt_signal!(),
    search_results_changed: qt_signal!(),
    data_refreshed: qt_signal!(),
    loading_changed: qt_signal!(),
    error_occurred: qt_signal!(message: String),
    page_saved: qt_signal!(),
    pdf_exported: qt_signal!(path: String),

    // Methods
    load_page: qt_method!(fn(&mut self, name: String)),
    save_block: qt_method!(fn(&mut self, index: i32, raw_text: String)),
    create_page: qt_method!(fn(&mut self, name: String)),
    delete_page: qt_method!(fn(&mut self, name: String)),
    do_search: qt_method!(fn(&mut self, query: String)),
    insert_link_at_cursor: qt_method!(fn(&mut self, block_idx: i32, cursor_pos: i32, target: String)),
    navigate_to_page: qt_method!(fn(&mut self, name: String)),
    load_main_page_data: qt_method!(fn(&mut self)),
    poll_results: qt_method!(fn(&mut self) -> bool),

    // Internal state
    conn: Option<rusqlite::Connection>,
    notes_dir: PathBuf,
    data_dir: PathBuf,
    current_blocks_data: Vec<Block>,
    pending: Arc<Mutex<Option<PendingResult>>>,
}

impl Default for FishdocBridge {
    fn default() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        let data_dir = PathBuf::from(&home).join(".local").join("share").join("harbour-fishdoc");
        let notes_dir = data_dir.join("notes");

        Self {
            base: Default::default(),
            current_page_name: String::new(),
            current_blocks: QVariantList::default(),
            is_journal_page: false,
            blocks_version: 0,
            search_query: String::new(),
            search_results: QVariantList::default(),
            recent_pages: QVariantList::default(),
            recent_journal_lines: QVariantList::default(),
            is_loading: false,
            error_message: String::new(),
            page_changed: Default::default(),
            search_results_changed: Default::default(),
            data_refreshed: Default::default(),
            loading_changed: Default::default(),
            error_occurred: Default::default(),
            page_saved: Default::default(),
            pdf_exported: Default::default(),
            load_page: Default::default(),
            save_block: Default::default(),
            create_page: Default::default(),
            delete_page: Default::default(),
            do_search: Default::default(),
            insert_link_at_cursor: Default::default(),
            navigate_to_page: Default::default(),
            load_main_page_data: Default::default(),
            poll_results: Default::default(),
            conn: None,
            notes_dir,
            data_dir,
            current_blocks_data: Vec::new(),
            pending: Arc::new(Mutex::new(None)),
        }
    }
}

impl FishdocBridge {
    fn ensure_init(&mut self) {
        if self.conn.is_some() {
            return;
        }

        let _ = std::fs::create_dir_all(&self.notes_dir);
        let _ = std::fs::create_dir_all(self.data_dir.join("exports"));

        let db_path = self.data_dir.join("fishdoc.db");
        match rusqlite::Connection::open(&db_path) {
            Ok(conn) => {
                if let Err(e) = db::init_schema(&conn) {
                    self.error_message = format!("DB init error: {}", e);
                    self.error_occurred(self.error_message.clone());
                    return;
                }
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
        let mut list = QVariantList::default();
        for block in blocks {
            let json = block.to_qvariant_map();
            let qv = QString::from(serde_json::to_string(&json).unwrap_or_default());
            list.push(qv.into());
        }
        list
    }
}
