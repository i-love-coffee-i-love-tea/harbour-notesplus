use qmetaobject::*;
use std::path::PathBuf;

use fishdoc_core::block::Block;
use fishdoc_core::db;
use fishdoc_core::journal;
use fishdoc_core::page;
use fishdoc_core::parser;
use fishdoc_core::search as search_mod;

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
        self.save_block_range_impl(index, 1, raw_text);
    }

    fn save_block_range_impl(&mut self, start_index: i32, count: i32, raw_text: String) {
        let start_idx = start_index as usize;
        let count = count as usize;

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

        let mut all = parser::parse_blocks_with_options(&content, self.drop_comments);
        if start_idx <= all.len() {
            let end_idx = (start_idx + count).min(all.len());
            let new_blocks = parser::parse_blocks_with_options(&raw_text, self.drop_comments);
            all.splice(start_idx..end_idx, new_blocks);
            let new_content = parser::blocks_to_adoc(&all);

            if let Err(e) = std::fs::write(&path, &new_content) {
                ::log::warn!("save_block_range: write failed: {}", e);
                return;
            }

            if let Some(conn) = self.conn() {
                if let Ok(Some(info)) = page::get_page(conn, &filename) {
                    let _ = db::update_fts_content(conn, info.id, &new_content);
                }
            }

            self.page_saved();
            let page_name = self.current_page_name.clone();
            if !page_name.is_empty() {
                self.load_page_impl(page_name);
            }
        }
    }

    fn append_to_current_page_impl(&mut self, text: String, is_task: bool) {
        self.ensure_init();
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return;
        }

        let is_journal = self.is_journal_page || self.current_page_name == "Journal" || self.current_page_name == "journal";
        let line_to_append = if is_task {
            if trimmed.starts_with("* [ ] ") || trimmed.starts_with("* [x] ") || trimmed.starts_with("* [X] ") {
                trimmed.to_string()
            } else if trimmed.starts_with("- [ ] ") || trimmed.starts_with("- [x] ") || trimmed.starts_with("- [X] ") {
                format!("* {}", &trimmed[2..])
            } else if let Some(stripped) = trimmed.strip_prefix("* ") {
                format!("* [ ] {}", stripped)
            } else if let Some(stripped) = trimmed.strip_prefix("- ") {
                format!("* [ ] {}", stripped)
            } else {
                format!("* [ ] {}", trimmed)
            }
        } else {
            trimmed.to_string()
        };

        if is_journal {
            if let Err(e) = journal::append_to_journal_today(&self.notes_path, &line_to_append) {
                self.error_message = format!("Failed to append to journal: {}", e);
                self.error_occurred(self.error_message.clone());
                return;
            }
            if let Some(conn) = self.conn() {
                let path = self.notes_path.join("journal.adoc");
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(Some(info)) = page::get_page(conn, "journal.adoc") {
                        let _ = db::update_fts_content(conn, info.id, &content);
                    }
                }
            }
            self.page_saved();
            self.load_page_impl("Journal".to_string());
            return;
        }

        let filename = self.resolve_page_filename(&self.current_page_name);
        let path = self.notes_path.join(&filename);

        let mut content = std::fs::read_to_string(&path).unwrap_or_default();

        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str(&line_to_append);
        content.push('\n');

        if let Err(e) = std::fs::write(&path, &content) {
            self.error_message = format!("Failed to append to note: {}", e);
            self.error_occurred(self.error_message.clone());
            return;
        }

        if let Some(conn) = self.conn() {
            if let Ok(Some(info)) = page::get_page(conn, &filename) {
                let _ = db::update_fts_content(conn, info.id, &content);
            }
        }

        self.page_saved();
        let page_name = self.current_page_name.clone();
        if !page_name.is_empty() {
            self.load_page_impl(page_name);
        }
    }

    fn save_journal_block_impl(&mut self, index: i32, raw_text: String) {
        let idx = index as usize;
        let filename = "journal.adoc".to_string();
        let path = self.notes_path.join(&filename);
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                ::log::warn!("save_journal_block: read failed: {}", e);
                return;
            }
        };

        let mut all = parser::parse_blocks_with_options(&content, self.drop_comments);
        if idx < all.len() {
            let new_blocks = parser::parse_blocks_with_options(&raw_text, self.drop_comments);
            all.splice(idx..idx + 1, new_blocks);
            let new_content = parser::blocks_to_adoc(&all);
            if let Err(e) = std::fs::write(&path, &new_content) {
                ::log::warn!("save_journal_block: write failed: {}", e);
                return;
            }

            if let Some(conn) = self.conn() {
                if let Ok(Some(info)) = page::get_page(conn, &filename) {
                    let _ = db::update_fts_content(conn, info.id, &new_content);
                }
            }

            self.page_saved();
            self.load_main_page_data_impl();
        }
    }

    fn toggle_journal_checkbox_impl(&mut self, block_index: i32, item_path: String) {
        let idx = block_index as usize;
        let filename = "journal.adoc".to_string();
        let path = self.notes_path.join(&filename);

        if idx < self.journal_blocks_data.len() {
            let mut curr = Some(&mut self.journal_blocks_data[idx]);
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
        }

        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return,
        };

        let mut blocks = parser::parse_blocks(&content);
        if idx < blocks.len() {
            let mut curr = Some(&mut blocks[idx]);
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
                let new_content = parser::blocks_to_adoc(&blocks);
                let _ = std::fs::write(&path, &new_content);
                if let Some(conn) = self.conn() {
                    if let Ok(Some(info)) = page::get_page(conn, &filename) {
                        let _ = db::update_fts_content(conn, info.id, &new_content);
                    }
                }
                self.page_saved();
                self.load_main_page_data_impl();
            }
        }
    }

    fn append_to_journal_impl(&mut self, text: String, is_task: bool) {
        self.ensure_init();
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return;
        }

        let line_to_append = if is_task {
            if trimmed.starts_with("* [ ] ") || trimmed.starts_with("* [x] ") || trimmed.starts_with("* [X] ") {
                trimmed.to_string()
            } else if trimmed.starts_with("- [ ] ") || trimmed.starts_with("- [x] ") || trimmed.starts_with("- [X] ") {
                format!("* {}", &trimmed[2..])
            } else if let Some(stripped) = trimmed.strip_prefix("* ") {
                format!("* [ ] {}", stripped)
            } else if let Some(stripped) = trimmed.strip_prefix("- ") {
                format!("* [ ] {}", stripped)
            } else {
                format!("* [ ] {}", trimmed)
            }
        } else {
            trimmed.to_string()
        };

        if let Err(e) = journal::append_to_journal_today(&self.notes_path, &line_to_append) {
            self.error_message = format!("Failed to append to journal: {}", e);
            self.error_occurred(self.error_message.clone());
            return;
        }

        if let Some(conn) = self.conn() {
            let path = self.notes_path.join("journal.adoc");
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(Some(info)) = page::get_page(conn, "journal.adoc") {
                    let _ = db::update_fts_content(conn, info.id, &content);
                }
            }
        }

        self.page_saved();
        self.load_main_page_data_impl();
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
                    let preview_values = page::get_page_preview_values_with_options(&self.notes_path, &p.filename, 8, self.drop_comments);
                    let preview_json_str = serde_json::to_string(&preview_values).unwrap_or_else(|_| "[]".to_string());

                    let mut map = serde_json::Map::new();
                    map.insert("name".into(), serde_json::Value::String(p.title.clone()));
                    map.insert("filename".into(), serde_json::Value::String(p.filename.clone()));
                    map.insert("created_at".into(), serde_json::Value::String(p.created_at.clone()));
                    map.insert("updated_at".into(), serde_json::Value::String(p.updated_at.clone()));
                    map.insert("block_count".into(), serde_json::Value::Number(p.block_count.into()));
                    map.insert("preview_blocks".into(), serde_json::Value::Array(preview_values));
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

        let _ = journal::init_journal(&self.notes_path);
        match journal::get_journal_blocks(&self.notes_path, Some(15), self.drop_comments) {
            Ok(blocks) => {
                let mut list = QVariantList::default();
                for b in &blocks {
                    let map = b.to_qvariant_map();
                    let json_str = serde_json::to_string(&map).unwrap_or_default();
                    list.push(QString::from(json_str).into());
                }
                self.journal_blocks = list;
                self.journal_blocks_data = blocks;
            }
            Err(e) => {
                ::log::warn!("Failed to load journal blocks: {}", e);
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

    fn export_html_impl(&mut self, page_name: String) -> String {
        self.ensure_init();
        let filename = if self.is_journal_page || page_name == "Journal" || page_name == "journal" {
            "journal.adoc".to_string()
        } else {
            self.resolve_page_filename(&page_name)
        };

        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        let export_dir = PathBuf::from(&home).join("Documents").join("Notes++ Exports");
        let title = page_name.strip_suffix(".adoc").unwrap_or(&page_name);
        let output_path = export_dir.join(format!("{}.html", title));

        match fishdoc_core::html::export_page_to_html5(&self.notes_path, &filename, &output_path) {
            Ok(path) => {
                let path_str = path.to_string_lossy().to_string();
                self.html_exported(path_str.clone());
                path_str
            }
            Err(e) => {
                self.error_message = format!("Export failed: {}", e);
                self.error_occurred(self.error_message.clone());
                String::new()
            }
        }
    }

    fn export_all_html_impl(&mut self) -> String {
        self.ensure_init();
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        let export_dir = PathBuf::from(&home).join("Documents").join("Notes++ Exports");
        let _ = std::fs::create_dir_all(&export_dir);

        let mut exported_count = 0;
        if let Ok(entries) = std::fs::read_dir(&self.notes_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                    if ext == "adoc" {
                        if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                            let title = filename.strip_suffix(".adoc").unwrap_or(filename);
                            let out_file = export_dir.join(format!("{}.html", title));
                            if fishdoc_core::html::export_page_to_html5(&self.notes_path, filename, &out_file).is_ok() {
                                exported_count += 1;
                            }
                        }
                    } else if ["png", "jpg", "jpeg", "svg", "gif", "webp"].contains(&ext.to_ascii_lowercase().as_str()) {
                        if let Some(filename) = path.file_name() {
                            let _ = std::fs::copy(&path, export_dir.join(filename));
                        }
                    }
                }
            }
        }

        let result_dir_str = export_dir.to_string_lossy().to_string();
        self.html_exported(format!("{} ({} notes)", result_dir_str, exported_count));
        result_dir_str
    }

    fn open_in_browser_impl(&mut self, page_name: String) {
        if self.web_server_running {
            let filename = if self.is_journal_page || page_name == "Journal" || page_name == "journal" {
                "journal.adoc".to_string()
            } else {
                self.resolve_page_filename(&page_name)
            };
            let port = self.server_handle.as_ref().map(|h| h.port()).unwrap_or(8080);
            let url = format!("http://127.0.0.1:{}/page/{}", port, filename);
            let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
        } else {
            let path_str = self.export_html_impl(page_name);
            if !path_str.is_empty() {
                let _ = std::process::Command::new("xdg-open").arg(&path_str).spawn();
            }
        }
    }

    fn start_web_server_impl(&mut self) -> String {
        self.ensure_init();
        if self.web_server_running {
            return self.web_server_url.clone();
        }

        match fishdoc_core::server::start_server(self.notes_path.clone(), 8080) {
            Ok(handle) => {
                let primary_url = handle.primary_url();
                self.web_server_url = primary_url.clone();
                self.web_server_running = true;
                self.server_handle = Some(handle);
                self.web_server_status_changed();
                primary_url
            }
            Err(e) => {
                self.error_message = format!("Failed to start web server: {}", e);
                self.error_occurred(self.error_message.clone());
                String::new()
            }
        }
    }

    fn stop_web_server_impl(&mut self) {
        if let Some(handle) = self.server_handle.take() {
            handle.stop();
        }
        self.web_server_running = false;
        self.web_server_url = String::new();
        self.web_server_status_changed();
    }

    fn toggle_web_server_impl(&mut self) -> bool {
        if self.web_server_running {
            self.stop_web_server_impl();
            false
        } else {
            !self.start_web_server_impl().is_empty()
        }
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

    fn get_linkable_pages_json_impl(&mut self, query: String) -> String {
        self.ensure_init();
        let conn = match self.conn() {
            Some(c) => c,
            None => return "[]".to_string(),
        };

        let query = query.trim();
        let mut results = Vec::new();
        let mut seen = std::collections::HashSet::new();

        if query.is_empty() {
            if let Ok(pages) = page::list_pages(conn) {
                for p in pages {
                    if seen.insert(p.filename.clone()) {
                        results.push(serde_json::json!({
                            "filename": p.filename,
                            "title": p.title,
                            "updated_at": p.updated_at,
                            "is_journal": p.is_journal,
                            "snippet": ""
                        }));
                    }
                }
            }
        } else {
            // Search pages using FTS
            if let Ok(search_results) = search_mod::search_pages(conn, query) {
                for r in search_results {
                    if seen.insert(r.page.filename.clone()) {
                        results.push(serde_json::json!({
                            "filename": r.page.filename,
                            "title": r.page.title,
                            "updated_at": r.page.updated_at,
                            "is_journal": r.page.is_journal,
                            "snippet": r.snippet
                        }));
                    }
                }
            }

            // Also search all pages for title / filename substring matching
            if let Ok(all_pages) = page::list_pages(conn) {
                let q_lower = query.to_lowercase();
                for p in all_pages {
                    if (p.title.to_lowercase().contains(&q_lower) || p.filename.to_lowercase().contains(&q_lower))
                        && seen.insert(p.filename.clone())
                    {
                        results.push(serde_json::json!({
                            "filename": p.filename,
                            "title": p.title,
                            "updated_at": p.updated_at,
                            "is_journal": p.is_journal,
                            "snippet": ""
                        }));
                    }
                }
            }
        }

        serde_json::to_string(&results).unwrap_or_else(|_| "[]".to_string())
    }

    // QML method wrappers
    pub fn get_linkable_pages_json(&mut self, query: String) -> String { self.get_linkable_pages_json_impl(query) }
    pub fn load_page(&mut self, name: String) { self.load_page_impl(name); }
    pub fn save_block(&mut self, index: i32, raw_text: String) { self.save_block_impl(index, raw_text); }
    pub fn save_block_range(&mut self, start_index: i32, count: i32, raw_text: String) { self.save_block_range_impl(start_index, count, raw_text); }
    pub fn append_to_current_page(&mut self, text: String, is_task: bool) { self.append_to_current_page_impl(text, is_task); }
    pub fn save_journal_block(&mut self, index: i32, raw_text: String) { self.save_journal_block_impl(index, raw_text); }
    pub fn toggle_journal_checkbox(&mut self, block_index: i32, item_path: String) { self.toggle_journal_checkbox_impl(block_index, item_path); }
    pub fn append_to_journal(&mut self, text: String, is_task: bool) { self.append_to_journal_impl(text, is_task); }
    pub fn get_page_source(&mut self, name: String) -> String { self.get_page_source_impl(name) }
    pub fn save_page_source(&mut self, name: String, content: String) { self.save_page_source_impl(name, content); }
    pub fn create_page(&mut self, name: String) { self.create_page_impl(name); }
    pub fn delete_page(&mut self, name: String) { self.delete_page_impl(name); }
    pub fn navigate_to_page(&mut self, name: String) { self.navigate_to_page_impl(name); }
    pub fn insert_link_at_cursor(&mut self, block_idx: i32, cursor_pos: i32, target: String) { self.insert_link_at_cursor_impl(block_idx, cursor_pos, target); }
    pub fn toggle_checkbox(&mut self, block_index: i32, item_path: String) { self.toggle_checkbox_impl(block_index, item_path); }
    pub fn set_drop_comments(&mut self, drop: bool) { self.set_drop_comments_impl(drop); }
    pub fn load_main_page_data(&mut self) { self.load_main_page_data_impl(); }
    pub fn export_html(&mut self, page_name: String) -> String { self.export_html_impl(page_name) }
    pub fn export_all_html(&mut self) -> String { self.export_all_html_impl() }
    pub fn open_in_browser(&mut self, page_name: String) { self.open_in_browser_impl(page_name); }
    pub fn start_web_server(&mut self) -> String { self.start_web_server_impl() }
    pub fn stop_web_server(&mut self) { self.stop_web_server_impl(); }
    pub fn toggle_web_server(&mut self) -> bool { self.toggle_web_server_impl() }
}
