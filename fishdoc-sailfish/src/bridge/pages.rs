use qmetaobject::*;

use fishdoc_core::journal;
use fishdoc_core::page;
use fishdoc_core::parser;
use fishdoc_core::search as search_mod;

use super::{FishdocBridge, PendingResult};

impl FishdocBridge {
    fn load_page_impl(&mut self, name: String) {
        self.ensure_init();
        let conn = match self.conn() {
            Some(c) => c,
            None => return,
        };

        let filename = if name == "Journal" || name == "journal" {
            "journal.adoc".to_string()
        } else {
            format!("{}.adoc", name)
        };

        match page::get_page(conn, &filename) {
            Ok(Some(info)) => {
                self.current_page_name = info.title.clone();
                self.is_journal_page = info.is_journal;

                let notes_dir = self.notes_dir.clone();
                let pending = self.pending.clone();
                self.is_loading = true;
                self.loading_changed();

                std::thread::spawn(move || {
                    let result = match page::read_page(&notes_dir, &filename) {
                        Ok(content) => {
                            let blocks = parser::parse_blocks(&content);
                            PendingResult {
                                blocks: Some(blocks),
                                error: None,
                            }
                        }
                        Err(e) => PendingResult {
                            blocks: None,
                            error: Some(e),
                        },
                    };
                    if let Ok(mut p) = pending.lock() {
                        *p = Some(result);
                    }
                });
            }
            Ok(None) => {
                self.error_message = format!("Page '{}' not found", name);
                self.error_occurred(self.error_message.clone());
            }
            Err(e) => {
                self.error_message = e;
                self.error_occurred(self.error_message.clone());
            }
        }
    }

    fn save_block_impl(&mut self, _index: i32, _raw_text: String) {
        // TODO: implement async save
        ::log::warn!("save_block called (not yet implemented)");
    }

    fn create_page_impl(&mut self, name: String) {
        self.ensure_init();
        let conn = match self.conn() {
            Some(c) => c,
            None => return,
        };

        match page::create_page(conn, &self.notes_dir, &name, false) {
            Ok(_) => {
                self.load_main_page_data_impl();
            }
            Err(e) => {
                self.error_message = e;
                self.error_occurred(self.error_message.clone());
            }
        }
    }

    fn delete_page_impl(&mut self, name: String) {
        self.ensure_init();
        let conn = match self.conn() {
            Some(c) => c,
            None => return,
        };

        let filename = format!("{}.adoc", name);
        match page::delete_page(conn, &self.notes_dir, &filename) {
            Ok(_) => {
                if self.current_page_name == name {
                    self.current_page_name.clear();
                    self.current_blocks_data.clear();
                    self.current_blocks = QVariantList::default();
                    self.page_changed();
                }
                self.load_main_page_data_impl();
            }
            Err(e) => {
                self.error_message = e;
                self.error_occurred(self.error_message.clone());
            }
        }
    }

    fn navigate_to_page_impl(&mut self, name: String) {
        self.load_page_impl(name);
    }

    fn insert_link_at_cursor_impl(&mut self, block_idx: i32, _cursor_pos: i32, target: String) {
        let idx = block_idx as usize;

        let filename = if self.is_journal_page {
            "journal.adoc".to_string()
        } else {
            format!("{}.adoc", self.current_page_name)
        };
        let path = self.notes_dir.join(&filename);

        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return,
        };
        let mut blocks = parser::parse_blocks(&content);

        if idx >= blocks.len() {
            return;
        }

        let xref = format!("xref:{}.adoc[{}]", target, target);
        let new_raw = format!("{} {}", blocks[idx].raw_text(), xref);
        let new_blocks = parser::parse_blocks(&new_raw);
        if let Some(new_block) = new_blocks.into_iter().next() {
            blocks[idx] = new_block;
            let new_content = parser::blocks_to_adoc(&blocks);
            let _ = std::fs::write(&path, &new_content);
        }
    }

    fn load_main_page_data_impl(&mut self) {
        self.ensure_init();
        let conn = match self.conn() {
            Some(c) => c,
            None => return,
        };

        match page::recent_pages(conn, 10) {
            Ok(pages) => {
                let mut list = QVariantList::default();
                for p in &pages {
                    let mut map = QVariantMap::default();
                    map.insert("name".into(), QString::from(p.title.clone()).into());
                    map.insert("filename".into(), QString::from(p.filename.clone()).into());
                    list.push(map.into());
                }
                self.recent_pages = list;
            }
            Err(e) => {
                ::log::warn!("Failed to load recent pages: {}", e);
            }
        }

        match journal::recent_journal_lines(&self.notes_dir, 5) {
            Ok(lines) => {
                let mut list = QVariantList::default();
                for line in &lines {
                    list.push(QString::from(line.clone()).into());
                }
                self.recent_journal_lines = list;
            }
            Err(e) => {
                ::log::warn!("Failed to load journal lines: {}", e);
            }
        }

        self.data_refreshed();
    }

    // QML method wrappers
    pub fn load_page(&mut self, name: String) { self.load_page_impl(name); }
    pub fn save_block(&mut self, index: i32, raw_text: String) { self.save_block_impl(index, raw_text); }
    pub fn create_page(&mut self, name: String) { self.create_page_impl(name); }
    pub fn delete_page(&mut self, name: String) { self.delete_page_impl(name); }
    pub fn navigate_to_page(&mut self, name: String) { self.navigate_to_page_impl(name); }
    pub fn insert_link_at_cursor(&mut self, block_idx: i32, cursor_pos: i32, target: String) { self.insert_link_at_cursor_impl(block_idx, cursor_pos, target); }
    pub fn load_main_page_data(&mut self) { self.load_main_page_data_impl(); }
}
