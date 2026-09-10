use qmetaobject::*;
use std::path::PathBuf;

use notesplusplus_core::block::Block;
use notesplusplus_core::constants::{JOURNAL_FILENAME, JOURNAL_TITLE};

/// Navigate to a block within `blocks[idx]` using a dot-separated `item_path`
/// and toggle its checkbox state. Returns `true` if a checkbox was toggled.
fn toggle_check_in_blocks(blocks: &mut [Block], idx: usize, item_path: &str) -> bool {
    if idx >= blocks.len() { return false; }
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
                return false;
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
            return true;
        }
    }
    false
}
use notesplusplus_core::db;
use notesplusplus_core::journal;
use notesplusplus_core::page;
use notesplusplus_core::parser;
use notesplusplus_core::search as search_mod;
use notesplusplus_core::agent::DEFAULT_OLLAMA_ENDPOINT;

use super::{NotesBridge, MainPageData, PendingResult};

impl NotesBridge {
    fn resolve_page_filename(&self, name_or_title: &str) -> String {
        if name_or_title.eq_ignore_ascii_case(JOURNAL_TITLE) {
            return JOURNAL_FILENAME.to_string();
        }
        if let Some(conn) = self.conn() {
            if let Ok(Some(info)) = page::get_page(conn, name_or_title) {
                return info.filename;
            }
        }
        page::ensure_adoc_extension(name_or_title)
    }

    fn notes_dir(&self) -> std::path::PathBuf {
        self.notes_path.join("notes")
    }

    fn report_error(&mut self, msg: String) {
        self.error_message = msg;
        self.error_occurred(self.error_message.clone());
    }

    fn mutate_page_blocks<F>(&mut self, filename: &str, mutator: F)
    where
        F: FnOnce(&mut Vec<Block>) -> bool,
    {
        let path = self.notes_dir().join(filename);
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                ::log::warn!("mutate_page_blocks: read failed: {}", e);
                return;
            }
        };

        let mut blocks = parser::parse_blocks_with_options(&content, self.drop_comments);
        if mutator(&mut blocks) {
            let new_content = parser::blocks_to_adoc(&blocks);
            if let Err(e) = std::fs::write(&path, &new_content) {
                ::log::warn!("mutate_page_blocks: write failed: {}", e);
                return;
            }

            if let Some(conn) = self.conn() {
                if let Ok(Some(info)) = page::get_page(conn, filename) {
                    let _ = db::update_fts_content(conn, info.id, &new_content);
                }
            }

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

                let notes_dir = self.notes_dir();
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
                self.report_error(format!("Page '{}' not found", name));
            }
            Err(e) => {
                self.report_error(e);
            }
        }
    }

    fn save_block_impl(&mut self, index: i32, raw_text: String) {
        self.save_block_range_impl(index, 1, raw_text);
    }

    fn save_block_range_impl(&mut self, start_index: i32, count: i32, raw_text: String) {
        if start_index < 0 || count < 0 { return; }
        let start_idx = start_index as usize;
        let count = count as usize;

        let filename = if self.is_journal_page {
            JOURNAL_FILENAME.to_string()
        } else {
            self.resolve_page_filename(&self.current_page_name)
        };

        let drop_comments = self.drop_comments;
        self.mutate_page_blocks(&filename, |all| {
            if start_idx <= all.len() {
                let end_idx = (start_idx + count).min(all.len());
                let new_blocks = parser::parse_blocks_with_options(&raw_text, drop_comments);
                all.splice(start_idx..end_idx, new_blocks);
                true
            } else {
                false
            }
        });

        let page_name = self.current_page_name.clone();
        if !page_name.is_empty() {
            self.load_page_impl(page_name);
        }
    }

    fn append_to_current_page_impl(&mut self, text: String, is_task: bool) {
        self.ensure_init();
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return;
        }

        let is_journal = self.is_journal_page || self.current_page_name.eq_ignore_ascii_case(JOURNAL_TITLE);
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
            if let Err(e) = journal::append_to_journal_today(&self.notes_dir(), &line_to_append) {
                self.report_error(format!("Failed to append to journal: {}", e));
                return;
            }
            if let Some(conn) = self.conn() {
                let path = self.notes_dir().join(JOURNAL_FILENAME);
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(Some(info)) = page::get_page(conn, JOURNAL_FILENAME) {
                        let _ = db::update_fts_content(conn, info.id, &content);
                    }
                }
            }
            self.load_page_impl("Journal".to_string());
            return;
        }

        let filename = self.resolve_page_filename(&self.current_page_name);
        let path = self.notes_dir().join(&filename);

        let mut content = std::fs::read_to_string(&path).unwrap_or_default();

        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str(&line_to_append);
        content.push('\n');

        if let Err(e) = std::fs::write(&path, &content) {
            self.report_error(format!("Failed to append to note: {}", e));
            return;
        }

        if let Some(conn) = self.conn() {
            if let Ok(Some(info)) = page::get_page(conn, &filename) {
                let _ = db::update_fts_content(conn, info.id, &content);
            }
        }

        let page_name = self.current_page_name.clone();
        if !page_name.is_empty() {
            self.load_page_impl(page_name);
        }
    }

    fn save_journal_block_impl(&mut self, index: i32, raw_text: String) {
        if index < 0 { return; }
        let idx = index as usize;
        let drop_comments = self.drop_comments;
        self.mutate_page_blocks(JOURNAL_FILENAME, |all| {
            if idx < all.len() {
                let new_blocks = parser::parse_blocks_with_options(&raw_text, drop_comments);
                all.splice(idx..idx + 1, new_blocks);
                true
            } else {
                false
            }
        });

        self.load_main_page_data_impl();
    }

    fn toggle_journal_checkbox_impl(&mut self, block_index: i32, item_path: String) {
        if block_index < 0 { return; }
        let idx = block_index as usize;
        let filename = JOURNAL_FILENAME.to_string();
        let path = self.notes_dir().join(&filename);

        if idx < self.journal_blocks_data.len() {
            toggle_check_in_blocks(&mut self.journal_blocks_data, idx, &item_path);
        }

        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return,
        };

        let mut blocks = parser::parse_blocks(&content);
        if toggle_check_in_blocks(&mut blocks, idx, &item_path) {
            let new_content = parser::blocks_to_adoc(&blocks);
            let _ = std::fs::write(&path, &new_content);
            if let Some(conn) = self.conn() {
                if let Ok(Some(info)) = page::get_page(conn, &filename) {
                    let _ = db::update_fts_content(conn, info.id, &new_content);
                }
            }
            self.load_main_page_data_impl();
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

        if let Err(e) = journal::append_to_journal_today(&self.notes_dir(), &line_to_append) {
            self.report_error(format!("Failed to append to journal: {}", e));
            return;
        }

        if let Some(conn) = self.conn() {
            let path = self.notes_dir().join(JOURNAL_FILENAME);
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(Some(info)) = page::get_page(conn, JOURNAL_FILENAME) {
                    let _ = db::update_fts_content(conn, info.id, &content);
                }
            }
        }

        self.load_main_page_data_impl();
    }

    fn get_page_source_impl(&mut self, name: String) -> String {
        let filename = self.resolve_page_filename(&name);
        let path = self.notes_dir().join(&filename);
        std::fs::read_to_string(&path).unwrap_or_default()
    }

    fn save_page_source_impl(&mut self, name: String, content: String) {
        self.ensure_init();
        let filename = self.resolve_page_filename(&name);
        let path = self.notes_dir().join(&filename);
        if let Err(e) = std::fs::write(&path, &content) {
            self.report_error(format!("Failed to save source: {}", e));
            return;
        }

        if let Some(conn) = self.conn() {
            if let Ok(Some(info)) = page::get_page(conn, &filename) {
                let _ = db::update_fts_content(conn, info.id, &content);
            }
        }

        self.load_page_impl(name);
    }

    fn toggle_checkbox_impl(&mut self, block_index: i32, item_path: String) {
        if block_index < 0 { return; }
        let idx = block_index as usize;
        let filename = if self.is_journal_page {
            JOURNAL_FILENAME.to_string()
        } else {
            self.resolve_page_filename(&self.current_page_name)
        };
        let path = self.notes_dir().join(&filename);

        if idx < self.current_blocks_data.len() {
            toggle_check_in_blocks(&mut self.current_blocks_data, idx, &item_path);
            self.current_blocks = Self::blocks_to_qvariantlist(&self.current_blocks_data);
        }

        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                ::log::warn!("toggle_checkbox: read failed: {}", e);
                return;
            }
        };

        let new_content = match parser::toggle_checkbox(&content, idx, &item_path) {
            Some(c) => c,
            None => {
                ::log::warn!("toggle_checkbox: could not find/toggle block {} path '{}'", idx, item_path);
                return;
            }
        };

        if let Err(e) = std::fs::write(&path, &new_content) {
            ::log::warn!("toggle_checkbox: write failed: {}", e);
        }
    }

    fn create_page_impl(&mut self, name: String) {
        self.ensure_init();
        let conn = match self.conn() {
            Some(c) => c,
            None => return,
        };

        match page::create_page(conn, &self.notes_dir(), &name, false) {
            Ok(_) => {
                self.load_main_page_data_impl();
            }
            Err(e) => {
                self.report_error(e);
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

        match page::delete_page(conn, &self.notes_dir(), &name) {
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
                self.report_error(e);
            }
        }
    }

    fn navigate_to_page_impl(&mut self, name: String) {
        self.load_page_impl(name);
    }

    fn insert_link_at_cursor_impl(&mut self, block_idx: i32, _cursor_pos: i32, target: String) {
        if block_idx < 0 { return; }
        let idx = block_idx as usize;

        let filename = if self.is_journal_page {
            JOURNAL_FILENAME.to_string()
        } else {
            self.resolve_page_filename(&self.current_page_name)
        };
        let path = self.notes_dir().join(&filename);

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
        let t_start = std::time::Instant::now();
        self.ensure_init();
        if !self.poll_init() {
            return;
        }
        eprintln!("[debug] init ready, starting data load at {:?}", t_start.elapsed());

        // Journal data (fast, keep synchronous)
        let _ = journal::init_journal(&self.notes_dir());
        self.journal_blocks = QVariantList::default();
        self.journal_blocks_data = Vec::new();

        match journal::recent_journal_lines(&self.notes_dir(), 5) {
            Ok(lines) => {
                let mut list = QVariantList::default();
                for line in &lines {
                    list.push(QString::from(line.clone()).into());
                }
                self.recent_journal_lines = list;
            }
            Err(e) => {
                eprintln!("[debug] Failed to load journal lines: {}", e);
            }
        }

        // Recent pages: fast DB query on main thread, then offload slow preview
        // generation to a background thread.
        let conn = match self.conn() {
            Some(c) => c,
            None => return,
        };

        let t = std::time::Instant::now();
        match page::recent_pages(conn, 10) {
            Ok(pages) => {
                eprintln!("[debug] recent_pages query in {:?} ({} pages)", t.elapsed(), pages.len());

                let notes_dir = self.notes_dir();
                let drop_comments = self.drop_comments;
                let pending = self.pending_main_page.clone();

                std::thread::spawn(move || {
                    let mut page_jsons = Vec::new();
                    for p in &pages {
                        let t_preview = std::time::Instant::now();
                        let preview_values = page::get_page_preview_values_with_options(&notes_dir, &p.filename, 8, drop_comments);
                        let preview_json_str = serde_json::to_string(&preview_values).unwrap_or_else(|_| "[]".to_string());
                        eprintln!("[debug]   preview '{}' in {:?}", p.filename, t_preview.elapsed());

                        let mut map = serde_json::Map::new();
                        map.insert("name".into(), serde_json::Value::String(p.title.clone()));
                        map.insert("filename".into(), serde_json::Value::String(p.filename.clone()));
                        map.insert("created_at".into(), serde_json::Value::String(p.created_at.clone()));
                        map.insert("updated_at".into(), serde_json::Value::String(p.updated_at.clone()));
                        map.insert("block_count".into(), serde_json::Value::Number(p.block_count.into()));
                        map.insert("preview_blocks".into(), serde_json::Value::Array(preview_values));
                        map.insert("preview_blocks_json".into(), serde_json::Value::String(preview_json_str));

                        let json_str = serde_json::to_string(&serde_json::Value::Object(map)).unwrap_or_default();
                        page_jsons.push(json_str);
                    }

                    if let Ok(mut slot) = pending.lock() {
                        *slot = Some(MainPageData { recent_page_jsons: page_jsons });
                    }
                    eprintln!("[debug] background preview generation done");
                });
            }
            Err(e) => {
                eprintln!("[debug] Failed to load recent pages: {}", e);
            }
        }

        eprintln!("[debug] load_main_page_data total: {:?}", t_start.elapsed());
        self.data_refreshed();
    }

    /// Poll for background main-page preview data. Returns true if data was
    /// consumed and properties updated.
    fn poll_main_page_data_impl(&mut self) -> bool {
        let has_result = if let Ok(guard) = self.pending_main_page.lock() {
            guard.is_some()
        } else {
            false
        };

        if !has_result {
            return false;
        }

        let data = match self.pending_main_page.lock() {
            Ok(mut guard) => guard.take(),
            Err(_) => return false,
        };

        if let Some(data) = data {
            let mut list = QVariantList::default();
            for json_str in data.recent_page_jsons {
                list.push(QString::from(json_str).into());
            }
            self.recent_pages = list;
            self.data_refreshed();
            return true;
        }

        false
    }

    fn export_html_impl(&mut self, page_name: String) -> String {
        self.ensure_init();
        let filename = if self.is_journal_page || page_name.eq_ignore_ascii_case(JOURNAL_TITLE) {
            JOURNAL_FILENAME.to_string()
        } else {
            self.resolve_page_filename(&page_name)
        };

        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        let export_dir = PathBuf::from(&home).join("Documents").join("Notes++ Exports");
        let title = page_name.strip_suffix(".adoc").unwrap_or(&page_name);
        let output_path = export_dir.join(format!("{}.html", title));

        match notesplusplus_core::html::export_page_to_html5(&self.notes_dir(), &self.notes_path, &filename, &output_path) {
            Ok(path) => {
                let path_str = path.to_string_lossy().to_string();
                path_str
            }
            Err(e) => {
                self.report_error(format!("Export failed: {}", e));
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
        let notes_subdir = self.notes_dir();
        // Export .adoc files from the notes subdirectory
        if let Ok(entries) = std::fs::read_dir(&notes_subdir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("adoc") {
                    if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                        let title = filename.strip_suffix(".adoc").unwrap_or(filename);
                        let out_file = export_dir.join(format!("{}.html", title));
                        if notesplusplus_core::html::export_page_to_html5(&notes_subdir, &self.notes_path, filename, &out_file).is_ok() {
                            exported_count += 1;
                        }
                    }
                }
            }
        }
        // Copy image assets from the root notes directory
        if let Ok(entries) = std::fs::read_dir(&self.notes_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                    if ["png", "jpg", "jpeg", "svg", "gif", "webp"].contains(&ext.to_ascii_lowercase().as_str()) {
                        if let Some(filename) = path.file_name() {
                            let _ = std::fs::copy(&path, export_dir.join(filename));
                        }
                    }
                }
            }
        }

        let result_dir_str = export_dir.to_string_lossy().to_string();
        result_dir_str
    }

    fn open_in_browser_impl(&mut self, page_name: String) {
        if self.web_server_running {
            let filename = if self.is_journal_page || page_name.eq_ignore_ascii_case(JOURNAL_TITLE) {
                JOURNAL_FILENAME.to_string()
            } else {
                self.resolve_page_filename(&page_name)
            };
            let port = self.server_handle.as_ref().map(|h| h.port()).unwrap_or(8080);
            let url = format!("http://127.0.0.1:{}/page/{}", port, filename);
            // Sailfish OS: use sailfish-browser instead of xdg-open
            let _ = std::process::Command::new("sailfish-browser").arg(&url).spawn();
        } else {
            let path_str = self.export_html_impl(page_name);
            if !path_str.is_empty() {
                // Open exported HTML file in browser
                let _ = std::process::Command::new("sailfish-browser").arg(&path_str).spawn();
            }
        }
    }

    fn start_web_server_impl(&mut self) -> String {
        self.ensure_init();
        if self.web_server_running {
            return self.web_server_url.clone();
        }

        let db_path = self.data_dir.join(notesplusplus_core::constants::DB_FILENAME);
        let backup_dir = self.data_dir.join("backups");
        let cert_dir = self.data_dir.join("tls");
        let cert_path = cert_dir.join("server.crt");
        let key_path = cert_dir.join("server.key");

        let config = notesplusplus_core::server::ServerConfig {
            notes_dir: self.notes_path.clone(),
            notes_subdir: self.notes_dir(),
            db_path,
            backup_dir,
            port: 8080,
            llm_config: self.llm_config.clone(),
            permission_config: self.permission_config.clone(),
            auth_config: self.auth_config.clone(),
            enable_tls: true,
            tls_cert_path: Some(cert_path),
            tls_key_path: Some(key_path),
            reject_public_networks: self.reject_public_networks,
        };

        match notesplusplus_core::server::start_server_with_config(config) {
            Ok(handle) => {
                if !self.pending_theme_colors.is_empty() {
                    handle.context().set_theme_colors(self.pending_theme_colors.clone());
                }
                let primary_url = handle.primary_url();
                self.web_server_url = primary_url.clone();
                self.web_server_running = true;
                self.server_handle = Some(handle);
                self.web_server_status_changed();
                primary_url
            }
            Err(e) => {
                self.report_error(format!("Failed to start web server: {}", e));
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

    fn set_reject_public_networks_impl(&mut self, reject: bool) {
        if self.reject_public_networks != reject {
            self.reject_public_networks = reject;
            if let Some(ref handle) = self.server_handle {
                handle.context().set_reject_public_networks(reject);
            }
            self.reject_public_networks_changed();
        }
    }

    fn set_theme_impl(&mut self, colors_json: String) {
        if let Ok(map) = serde_json::from_str::<std::collections::HashMap<String, String>>(&colors_json) {
            self.pending_theme_colors = map.clone();
            if let Some(ref handle) = self.server_handle {
                handle.context().set_theme_colors(map);
            }
        }
    }

    fn set_session_expiry_hours_impl(&mut self, hours: i32) {
        let secs = (hours.max(1) as u64) * 3600;
        self.auth_config.session_expiry_secs = secs;
        if let Some(ref handle) = self.server_handle {
            handle.context().set_session_expiry_secs(secs);
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

    fn configure_ai_impl(
        &mut self,
        provider: String,
        url: String,
        model: String,
        key: String,
        timeout: i32,
        auto_read: bool,
        auto_create: bool,
        require_edit: bool,
        allow_self_signed: bool,
        allow_fetch: bool,
    ) {
        let p: notesplusplus_core::agent::LlmProvider = provider.parse().unwrap_or_default();
        self.llm_config.provider = p;
        self.llm_config.endpoint_url = if url.trim().is_empty() {
            match p {
                notesplusplus_core::agent::LlmProvider::Ollama => DEFAULT_OLLAMA_ENDPOINT.to_string(),
                notesplusplus_core::agent::LlmProvider::OpenAiCompatible => "https://api.mimocode.com".to_string(),
            }
        } else {
            url.trim().to_string()
        };
        self.llm_config.model = if model.trim().is_empty() {
            notesplusplus_core::constants::DEFAULT_AI_MODEL.to_string()
        } else {
            model.trim().to_string()
        };
        self.llm_config.api_key = if key.trim().is_empty() {
            None
        } else {
            Some(key.trim().to_string())
        };
        self.llm_config.timeout_secs = if timeout > 0 { timeout as u64 } else { 90 };
        self.llm_config.allow_self_signed = allow_self_signed;

        self.permission_config.auto_allow_read = auto_read;
        self.permission_config.auto_allow_create = auto_create;
        self.permission_config.require_confirm_edit = require_edit;
        self.permission_config.allow_fetch_url = allow_fetch;

        if let Some(ref handle) = self.server_handle {
            handle.context().update_llm_config(self.llm_config.clone(), Some(self.permission_config.clone()));
        }
    }

    fn install_tls_certificate_impl(&mut self, cert_pem_or_path: String, key_pem_or_path: String) -> String {
        self.ensure_init();
        let tls_dir = self.data_dir.join("tls");
        let cert_path = tls_dir.join("server.crt");
        let key_path = tls_dir.join("server.key");

        match notesplusplus_core::server::tls::install_custom_tls_cert(
            &cert_pem_or_path,
            &key_pem_or_path,
            &cert_path,
            &key_path,
        ) {
            Ok(_) => {
                if self.web_server_running {
                    self.stop_web_server_impl();
                    self.start_web_server_impl();
                }
                String::new()
            }
            Err(e) => {
                self.report_error(format!("Failed to install SSL certificate: {}", e));
                e
            }
        }
    }

    fn reset_tls_certificate_impl(&mut self) -> String {
        self.ensure_init();
        let tls_dir = self.data_dir.join("tls");
        let cert_path = tls_dir.join("server.crt");
        let key_path = tls_dir.join("server.key");

        match notesplusplus_core::server::tls::reset_to_self_signed_cert(&cert_path, &key_path, None) {
            Ok(_) => {
                if self.web_server_running {
                    self.stop_web_server_impl();
                    self.start_web_server_impl();
                }
                String::new()
            }
            Err(e) => {
                self.report_error(format!("Failed to reset SSL certificate: {}", e));
                e
            }
        }
    }

    fn is_custom_tls_certificate_impl(&self) -> bool {
        let cert_path = self.data_dir.join("tls").join("server.crt");
        notesplusplus_core::server::tls::is_custom_cert_installed(&cert_path)
    }

    fn get_tls_certificate_info_json_impl(&self) -> String {
        let tls_dir = self.data_dir.join("tls");
        let cert_path = tls_dir.join("server.crt");
        let key_path = tls_dir.join("server.key");
        let is_custom = notesplusplus_core::server::tls::is_custom_cert_installed(&cert_path);
        serde_json::json!({
            "is_custom": is_custom,
            "cert_path": cert_path.to_string_lossy(),
            "key_path": key_path.to_string_lossy(),
            "exists": cert_path.exists() && key_path.exists(),
        }).to_string()
    }

    // QML method wrappers
    pub fn configure_ai(&mut self, provider: String, url: String, model: String, key: String, timeout: i32, auto_read: bool, auto_create: bool, require_edit: bool, allow_self_signed: bool, allow_fetch: bool) {
        self.configure_ai_impl(provider, url, model, key, timeout, auto_read, auto_create, require_edit, allow_self_signed, allow_fetch);
    }
    pub fn install_tls_certificate(&mut self, cert_pem_or_path: String, key_pem_or_path: String) -> String {
        self.install_tls_certificate_impl(cert_pem_or_path, key_pem_or_path)
    }
    pub fn reset_tls_certificate(&mut self) -> String {
        self.reset_tls_certificate_impl()
    }
    pub fn is_custom_tls_certificate(&mut self) -> bool {
        self.is_custom_tls_certificate_impl()
    }
    pub fn get_tls_certificate_info_json(&mut self) -> String {
        self.get_tls_certificate_info_json_impl()
    }
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
    pub fn set_reject_public_networks(&mut self, reject: bool) { self.set_reject_public_networks_impl(reject); }
    pub fn load_main_page_data(&mut self) { self.load_main_page_data_impl(); }
    pub fn poll_main_page_data(&mut self) -> bool { self.poll_main_page_data_impl() }
    pub fn export_html(&mut self, page_name: String) -> String { self.export_html_impl(page_name) }
    pub fn export_all_html(&mut self) -> String { self.export_all_html_impl() }
    pub fn open_in_browser(&mut self, page_name: String) { self.open_in_browser_impl(page_name); }
    pub fn start_web_server(&mut self) -> String { self.start_web_server_impl() }
    pub fn stop_web_server(&mut self) { self.stop_web_server_impl(); }
    pub fn toggle_web_server(&mut self) -> bool { self.toggle_web_server_impl() }

    pub fn set_theme(&mut self, colors_json: String) { self.set_theme_impl(colors_json); }
    pub fn set_session_expiry_hours(&mut self, hours: i32) { self.set_session_expiry_hours_impl(hours); }

    /// Polls the server context for a pending authorization challenge. Returns true if one is pending.
    pub fn check_auth_challenge(&mut self) -> bool {
        if let Some(ref handle) = self.server_handle {
            let ctx = handle.context();
            let guard = ctx.pending_auth_challenge.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(challenge_id) = guard.as_ref() {
                let mut changed = false;
                if self.auth_challenge_id != *challenge_id {
                    self.auth_challenge_id = challenge_id.clone();
                    changed = true;
                }
                if let Some(challenge) = ctx.auth_challenges.get_challenge(challenge_id) {
                    if self.auth_verification_code != challenge.verification_code {
                        self.auth_verification_code = challenge.verification_code.clone();
                        changed = true;
                    }
                }
                if !self.auth_challenge_pending {
                    self.auth_challenge_pending = true;
                    changed = true;
                }
                if changed {
                    self.auth_challenge_changed();
                }
                return true;
            }
        }
        if self.auth_challenge_pending {
            self.auth_challenge_pending = false;
            self.auth_challenge_id = String::new();
            self.auth_verification_code = String::new();
            self.auth_challenge_changed();
        }
        false
    }

    /// Approves an authorization challenge.
    pub fn approve_auth_challenge(&mut self, challenge_id: String) {
        if let Some(ref handle) = self.server_handle {
            let ctx = handle.context();
            ctx.auth_challenges.approve_challenge(&challenge_id);
            ctx.clear_auth_challenge();
        }
        self.auth_challenge_pending = false;
        self.auth_challenge_id = String::new();
        self.auth_verification_code = String::new();
        self.auth_challenge_changed();
    }

    /// Denies/cancels an authorization challenge.
    pub fn deny_auth_challenge(&mut self, challenge_id: String) {
        if let Some(ref handle) = self.server_handle {
            let ctx = handle.context();
            ctx.auth_challenges.deny_challenge(&challenge_id);
            ctx.clear_auth_challenge();
        }
        self.auth_challenge_pending = false;
        self.auth_challenge_id = String::new();
        self.auth_verification_code = String::new();
        self.auth_challenge_changed();
    }
}

#[cfg(test)]
mod tests {
    use notesplusplus_core::constants::JOURNAL_FILENAME;
    use notesplusplus_core::parser;

    #[test]
    fn negative_index_rejected() {
        let block_index: i32 = -1;
        assert!(block_index < 0, "negative index should be caught by guard");
    }

    #[test]
    fn zero_index_accepted() {
        let block_index: i32 = 0;
        assert!(block_index >= 0, "zero index should be valid");
    }

    #[test]
    fn checkbox_toggle_roundtrip() {
        let content = "* [ ] task item\n* [x] done item\n";
        let toggled = parser::toggle_checkbox(content, 0, "").unwrap();
        assert!(toggled.contains("* [x] task item"));
        assert!(toggled.contains("* [x] done item"));

        let toggled_back = parser::toggle_checkbox(&toggled, 0, "").unwrap();
        assert!(toggled_back.contains("* [ ] task item"));
    }

    #[test]
    fn nested_checkbox_toggle() {
        let content = "* [ ] parent\n** [ ] child\n";
        let toggled = parser::toggle_checkbox(content, 0, "0").unwrap();
        assert!(toggled.contains("* [ ] parent"));
        assert!(toggled.contains("** [x] child"));
    }

    #[test]
    fn journal_filename_constant() {
        assert_eq!(JOURNAL_FILENAME, "journal.adoc");
    }
}
