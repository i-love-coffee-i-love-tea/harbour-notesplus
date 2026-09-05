use qmetaobject::*;

use fishdoc_core::block::Block;
use fishdoc_core::db;
use fishdoc_core::journal;
use fishdoc_core::page;
use fishdoc_core::parser;

use super::{FishdocBridge, PendingResult};

impl FishdocBridge {
    fn resolve_page_filename(&self, name_or_title: &str) -> String {
        if name_or_title == "Journal" || name_or_title == "journal" {
            return "journal.adoc".to_string();
        }
        if let Some(conn) = self.conn() {
            if let Ok(Some(info)) = page::get_page(conn, name_or_title) {
                return info.filename;
            }
        }
        if name_or_title.ends_with(".adoc") {
            name_or_title.to_string()
        } else {
            format!("{}.adoc", name_or_title)
        }
    }

    fn load_page_impl(&mut self, name: String) {
        self.ensure_init();
        let conn = match self.conn() {
            Some(c) => c,
            None => return,
        };

        match page::get_page(conn, &name) {
            Ok(Some(info)) => {
                let filename = info.filename.clone();
                self.current_page_name = info.title.clone();
                self.is_journal_page = info.is_journal;

                let notes_dir = self.notes_path.clone();
                let pending = self.pending.clone();
                let drop_comments = self.drop_comments;
                self.is_loading = true;
                self.loading_changed();

                std::thread::spawn(move || {
                    let result = match page::read_page(&notes_dir, &filename) {
                        Ok(content) => {
                            let blocks = parser::parse_blocks_with_options(&content, drop_comments);
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

    fn save_block_impl(&mut self, index: i32, raw_text: String) {
        let idx = index as usize;
        if idx >= self.current_blocks_data.len() {
            ::log::warn!("save_block: index {} out of range", idx);
            return;
        }

        let filename = if self.is_journal_page {
            "journal.adoc".to_string()
        } else {
            self.resolve_page_filename(&self.current_page_name)
        };
        let path = self.notes_path.join(&filename);

        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                ::log::warn!("save_block: read failed: {}", e);
                return;
            }
        };

        let old_raw = self.current_blocks_data[idx].raw_text();
        let old_lines: Vec<&str> = content.lines().collect();
        let mut new_lines: Vec<String> = Vec::with_capacity(old_lines.len());
        let mut replaced = false;
        for line in &old_lines {
            if !replaced && *line == old_raw {
                new_lines.push(raw_text.clone());
                replaced = true;
            } else {
                new_lines.push(line.to_string());
            }
        }

        if !replaced {
            ::log::warn!("save_block: could not find block {} raw text in file", idx);
            return;
        }

        let new_content = new_lines.join("\n");
        if let Err(e) = std::fs::write(&path, &new_content) {
            ::log::warn!("save_block: write failed: {}", e);
            return;
        }

        if let Some(conn) = self.conn() {
            if let Ok(Some(info)) = page::get_page(conn, &filename) {
                let _ = db::update_fts_content(conn, info.id, &new_content);
            }
        }
    }

    fn save_block_range_impl(&mut self, start_index: i32, count: i32, raw_text: String) {
        let start_idx = start_index as usize;
        let count = count as usize;
        if start_idx >= self.current_blocks_data.len() {
            ::log::warn!("save_block_range: index {} out of range", start_idx);
            return;
        }

        let end_idx = (start_idx + count).min(self.current_blocks_data.len());
        let filename = if self.is_journal_page {
            "journal.adoc".to_string()
        } else {
            self.resolve_page_filename(&self.current_page_name)
        };
        let path = self.notes_path.join(&filename);

        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                ::log::warn!("save_block_range: read failed: {}", e);
                return;
            }
        };

        let mut old_raws = Vec::new();
        for b in &self.current_blocks_data[start_idx..end_idx] {
            old_raws.push(b.raw_text());
        }
        let old_combined = old_raws.join("\n");

        let new_content = if content.contains(&old_combined) {
            content.replacen(&old_combined, &raw_text, 1)
        } else {
            let mut all = parser::parse_blocks(&content);
            if start_idx < all.len() {
                let end = (start_idx + count).min(all.len());
                let new_blocks = parser::parse_blocks(&raw_text);
                all.splice(start_idx..end, new_blocks);
                parser::blocks_to_adoc(&all)
            } else {
                content
            }
        };

        if let Err(e) = std::fs::write(&path, &new_content) {
            ::log::warn!("save_block_range: write failed: {}", e);
            return;
        }

        if let Some(conn) = self.conn() {
            if let Ok(Some(info)) = page::get_page(conn, &filename) {
                let _ = db::update_fts_content(conn, info.id, &new_content);
            }
        }
    }

    fn get_page_source_impl(&mut self, name: String) -> String {
        let filename = self.resolve_page_filename(&name);
        let path = self.notes_path.join(&filename);
        std::fs::read_to_string(&path).unwrap_or_default()
    }

    fn save_page_source_impl(&mut self, name: String, content: String) {
        self.ensure_init();
        let filename = self.resolve_page_filename(&name);
        let path = self.notes_path.join(&filename);
        if let Err(e) = std::fs::write(&path, &content) {
            self.error_message = format!("Failed to save source: {}", e);
            self.error_occurred(self.error_message.clone());
            return;
        }

        if let Some(conn) = self.conn() {
            if let Ok(Some(info)) = page::get_page(conn, &filename) {
                let _ = db::update_fts_content(conn, info.id, &content);
            }
        }

        self.page_saved();
        self.load_page_impl(name);
    }

    fn toggle_checkbox_impl(&mut self, block_index: i32, item_path: String) {
        let idx = block_index as usize;
        let filename = if self.is_journal_page {
            "journal.adoc".to_string()
        } else {
            self.resolve_page_filename(&self.current_page_name)
        };
        let path = self.notes_path.join(&filename);

        // Update in-memory block data cache synchronously without resetting page model
        if idx < self.current_blocks_data.len() {
            let mut curr = Some(&mut self.current_blocks_data[idx]);
            if !item_path.is_empty() {
                for part in item_path.split('.') {
                    if let Ok(child_idx) = part.parse::<usize>() {
                        curr = match curr {
                            Some(Block::UnorderedListItem { ref mut children, .. }) => children.get_mut(child_idx),
                            Some(Block::OrderedListItem { ref mut children, .. }) => children.get_mut(child_idx),
                            _ => None,
                        };
                    } else {
                        curr = None;
                        break;
                    }
                }
            }
            if let Some(Block::UnorderedListItem { ref mut checked, ref mut raw, .. }) = curr {
                if let Some(c) = checked {
                    *checked = Some(!*c);
                    if raw.contains("[ ] ") {
                        *raw = raw.replacen("[ ] ", "[x] ", 1);
                    } else if raw.contains("[x] ") {
                        *raw = raw.replacen("[x] ", "[ ] ", 1);
                    } else if raw.contains("[X] ") {
                        *raw = raw.replacen("[X] ", "[ ] ", 1);
                    }
                }
            }
            self.current_blocks = Self::blocks_to_qvariantlist(&self.current_blocks_data);
        }

        let item_path_clone = item_path.clone();
        std::thread::spawn(move || {
            let content = match std::fs::read_to_string(&path) {
                Ok(c) => c,
                Err(e) => {
                    ::log::warn!("toggle_checkbox: read failed: {}", e);
                    return;
                }
            };

            let new_content = match parser::toggle_checkbox(&content, idx, &item_path_clone) {
                Some(c) => c,
                None => {
                    ::log::warn!("toggle_checkbox: could not find/toggle block {} path '{}'", idx, item_path_clone);
                    return;
                }
            };

            if let Err(e) = std::fs::write(&path, &new_content) {
                ::log::warn!("toggle_checkbox: write failed: {}", e);
            }
        });
    }

    fn create_page_impl(&mut self, name: String) {
        self.ensure_init();
        let conn = match self.conn() {
            Some(c) => c,
            None => return,
        };

        match page::create_page(conn, &self.notes_path, &name, false) {
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

        let target_page = page::get_page(conn, &name).ok().flatten();
        let target_title = target_page.as_ref().map(|p| p.title.clone()).unwrap_or_else(|| name.clone());
        let target_filename = target_page.as_ref().map(|p| p.filename.clone());

        match page::delete_page(conn, &self.notes_path, &name) {
            Ok(_) => {
                let is_current = self.current_page_name == name
                    || self.current_page_name == target_title
                    || target_filename.as_ref().map_or(false, |f| f == &format!("{}.adoc", self.current_page_name))
                    || self.current_page_name.is_empty();

                if is_current {
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
            self.resolve_page_filename(&self.current_page_name)
        };
        let path = self.notes_path.join(&filename);

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
                    let preview_json_str = page::get_page_preview_json_with_options(&self.notes_path, &p.filename, 10, self.drop_comments);

                    let mut map = serde_json::Map::new();
                    map.insert("name".into(), serde_json::Value::String(p.title.clone()));
                    map.insert("filename".into(), serde_json::Value::String(p.filename.clone()));
                    map.insert("created_at".into(), serde_json::Value::String(p.created_at.clone()));
                    map.insert("updated_at".into(), serde_json::Value::String(p.updated_at.clone()));
                    map.insert("block_count".into(), serde_json::Value::Number(p.block_count.into()));
                    map.insert("preview_blocks_json".into(), serde_json::Value::String(preview_json_str));

                    let json_str = serde_json::to_string(&serde_json::Value::Object(map)).unwrap_or_default();
                    list.push(QString::from(json_str).into());
                }
                self.recent_pages = list;
            }
            Err(e) => {
                ::log::warn!("Failed to load recent pages: {}", e);
            }
        }

        match journal::recent_journal_lines(&self.notes_path, 5) {
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

    fn export_pdf_impl(&mut self, _page_name: String) {
        self.error_message = "PDF export is not available in this build".to_string();
        self.error_occurred(self.error_message.clone());
    }

    fn set_drop_comments_impl(&mut self, drop: bool) {
        if self.drop_comments != drop {
            self.drop_comments = drop;
            self.drop_comments_changed();
            if !self.current_page_name.is_empty() {
                let name = self.current_page_name.clone();
                self.load_page_impl(name);
            }
            self.load_main_page_data_impl();
        }
    }

    // QML method wrappers
    pub fn load_page(&mut self, name: String) { self.load_page_impl(name); }
    pub fn save_block(&mut self, index: i32, raw_text: String) { self.save_block_impl(index, raw_text); }
    pub fn save_block_range(&mut self, start_index: i32, count: i32, raw_text: String) { self.save_block_range_impl(start_index, count, raw_text); }
    pub fn get_page_source(&mut self, name: String) -> String { self.get_page_source_impl(name) }
    pub fn save_page_source(&mut self, name: String, content: String) { self.save_page_source_impl(name, content); }
    pub fn create_page(&mut self, name: String) { self.create_page_impl(name); }
    pub fn delete_page(&mut self, name: String) { self.delete_page_impl(name); }
    pub fn navigate_to_page(&mut self, name: String) { self.navigate_to_page_impl(name); }
    pub fn insert_link_at_cursor(&mut self, block_idx: i32, cursor_pos: i32, target: String) { self.insert_link_at_cursor_impl(block_idx, cursor_pos, target); }
    pub fn toggle_checkbox(&mut self, block_index: i32, item_path: String) { self.toggle_checkbox_impl(block_index, item_path); }
    pub fn set_drop_comments(&mut self, drop: bool) { self.set_drop_comments_impl(drop); }
    pub fn load_main_page_data(&mut self) { self.load_main_page_data_impl(); }
    pub fn export_pdf(&mut self, page_name: String) { self.export_pdf_impl(page_name); }
}
