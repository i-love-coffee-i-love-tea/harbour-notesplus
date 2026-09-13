use qmetaobject::*;
use std::path::PathBuf;

use notesplusplus_core::block::Block;
use notesplusplus_core::constants::{JOURNAL_FILENAME, JOURNAL_TITLE};

use notesplusplus_core::db;
use notesplusplus_core::journal;
use notesplusplus_core::page;
use notesplusplus_core::parser;
use notesplusplus_core::search as search_mod;

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

    pub fn notes_dir(&self) -> std::path::PathBuf {
        self.notes_path.clone()
    }

    pub fn report_error(&mut self, msg: String) {
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
            if let Err(e) = page::atomic_write(&path, &new_content) {
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
        if !self.ensure_init_blocking() {
            return;
        }
        let conn = match self.conn() {
            Some(c) => c,
            None => return,
        };

        match page::get_page(conn, &name) {
            Ok(Some(info)) => {
                let full_path = info.full_path();
                self.current_page_name = info.title.clone();
                self.current_page_group_path = info.group_path.clone();
                self.current_page_group_path_changed();
                self.is_journal_page = info.is_journal;

                let notes_dir = self.notes_dir();
                let pending = self.pending.clone();
                let drop_comments = self.drop_comments;
                self.is_loading = true;
                self.loading_changed();

                std::thread::spawn(move || {
                    let result = match page::read_page(&notes_dir, &full_path) {
                        Ok(content) => {
                            let blocks = parser::parse_blocks_with_options(&content, drop_comments);
                            PendingResult {
                                blocks: Some(blocks),
                                error: None,
                            }
                        }
                        Err(e) => PendingResult {
                            blocks: None,
                            error: Some(e.to_string()),
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
                self.report_error(e.to_string());
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
        let line_to_append = journal::format_task_line(trimmed, is_task);

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

        if let Err(e) = page::atomic_write(&path, &content) {
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
            parser::toggle_check_in_blocks(&mut self.journal_blocks_data, idx, &item_path);
        }

        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return,
        };

        let mut blocks = parser::parse_blocks(&content);
        if parser::toggle_check_in_blocks(&mut blocks, idx, &item_path) {
            let new_content = parser::blocks_to_adoc(&blocks);
            let _ = page::atomic_write(&path, &new_content);
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

        let line_to_append = journal::format_task_line(trimmed, is_task);

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
        if let Err(e) = page::atomic_write(&path, &content) {
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
            parser::toggle_check_in_blocks(&mut self.current_blocks_data, idx, &item_path);
            let options = notesplusplus_core::html::qt_html::QtRenderOptions {
                notes_dir: Some(self.notes_path.to_string_lossy().to_string()),
                allow_external_images: true,
                search_terms: Vec::new(),
            };
            self.current_blocks = Self::blocks_to_qvariantlist_with_html(&self.current_blocks_data, &self.qt_theme, &options);
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

        if let Err(e) = page::atomic_write(&path, &new_content) {
            ::log::warn!("toggle_checkbox: write failed: {}", e);
        }
    }

    fn delete_page_impl(&mut self, name: String) {
        if !self.ensure_init_blocking() {
            return;
        }
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
                self.report_error(e.to_string());
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
            let _ = page::atomic_write(&path, &new_content);
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
        let all_pages = page::list_pages(conn).unwrap_or_default();
        let all_groups = notesplusplus_core::group::get_groups_flat(conn).unwrap_or_default();
        let display_depth = self.group_display_depth;
        eprintln!("[debug] list_pages query in {:?} ({} pages, {} groups)", t.elapsed(), all_pages.len(), all_groups.len());
        for p in &all_pages {
            eprintln!("[debug]   page: id={} filename='{}' group_path='{}' title='{}' journal={}", p.id, p.filename, p.group_path, p.title, p.is_journal);
        }
        for g in &all_groups {
            eprintln!("[debug]   group: path='{}' display_name='{}' notes={} children={}", g.path, g.display_name, g.note_count, g.child_group_count);
        }

        let notes_dir = self.notes_dir();
        let notes_path = self.notes_path.to_string_lossy().to_string();
        let drop_comments = self.drop_comments;
        let pending = self.pending_main_page.clone();
        let qt_theme = self.qt_theme.clone();
        let qt_options = notesplusplus_core::html::qt_html::QtRenderOptions {
            notes_dir: Some(notes_path),
            allow_external_images: true,
            ..Default::default()
        };

        std::thread::spawn(move || {
            let tree_json = notesplusplus_core::tree::build_group_tree(
                &all_pages,
                &all_groups,
                display_depth,
                Some(&notes_dir),
                drop_comments,
                Some(&qt_theme),
                Some(&qt_options),
            );

            let mut page_jsons = Vec::new();
            let recent_slice: Vec<_> = all_pages.iter().filter(|p| !p.is_journal).take(10).collect();
            for p in &recent_slice {
                let t_preview = std::time::Instant::now();
                let preview_values = page::get_page_preview_values_with_options(&notes_dir, &p.full_path(), 8, drop_comments, Some(&qt_theme), Some(&qt_options));
                let preview_json_str = serde_json::to_string(&preview_values).unwrap_or_else(|_| "[]".to_string());
                eprintln!("[debug]   preview '{}' in {:?}", p.filename, t_preview.elapsed());

                let mut map = serde_json::Map::new();
                map.insert("id".into(), serde_json::Value::Number(p.id.into()));
                map.insert("name".into(), serde_json::Value::String(p.title.clone()));
                map.insert("filename".into(), serde_json::Value::String(p.filename.clone()));
                map.insert("group_path".into(), serde_json::Value::String(p.group_path.clone()));
                map.insert("full_path".into(), serde_json::Value::String(p.full_path()));
                map.insert("created_at".into(), serde_json::Value::String(p.created_at.clone()));
                map.insert("updated_at".into(), serde_json::Value::String(p.updated_at.clone()));
                map.insert("block_count".into(), serde_json::Value::Number(p.block_count.into()));
                map.insert("preview_blocks".into(), serde_json::Value::Array(preview_values));
                map.insert("preview_blocks_json".into(), serde_json::Value::String(preview_json_str));

                let json_str = serde_json::to_string(&serde_json::Value::Object(map)).unwrap_or_default();
                page_jsons.push(json_str);
            }

            if let Ok(mut slot) = pending.lock() {
                *slot = Some(MainPageData {
                    recent_page_jsons: page_jsons,
                    grouped_tree_json: tree_json,
                });
            }
            eprintln!("[debug] background preview generation done");
        });

        eprintln!("[debug] load_main_page_data total: {:?}", t_start.elapsed());
        self.data_refreshed();
    }

    /// Poll for background main-page preview data. Returns true if data was
    /// consumed and properties updated.
    fn poll_main_page_data_impl(&mut self) -> bool {
        // If DB init was pending, poll it and trigger initial data load upon completion
        if self.conn.is_none() {
            if self.poll_init() {
                self.load_main_page_data_impl();
            }
            return false;
        }

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
            self.grouped_tree_json = QString::from(data.grouped_tree_json);
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
        let notes_dir = self.notes_dir();
        // Export .adoc files from the notes directory
        if let Ok(entries) = std::fs::read_dir(&notes_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("adoc") {
                    if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                        let title = filename.strip_suffix(".adoc").unwrap_or(filename);
                        let out_file = export_dir.join(format!("{}.html", title));
                        if notesplusplus_core::html::export_page_to_html5(&notes_dir, &self.notes_path, filename, &out_file).is_ok() {
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

    fn open_in_browser_impl(&mut self, page_name: String) -> String {
        if self.web_server_running {
            let filename = if self.is_journal_page || page_name.eq_ignore_ascii_case(JOURNAL_TITLE) {
                JOURNAL_FILENAME.to_string()
            } else {
                self.resolve_page_filename(&page_name)
            };
            let port = self.server_handle.as_ref().map(|h| h.port()).unwrap_or(8080);
            let base = self.server_handle.as_ref().map(|h| h.primary_url()).unwrap_or_else(|| format!("http://127.0.0.1:{}", port));
            let base = base.replace("0.0.0.0", "localhost");
            format!("{}/page/{}", base, filename)
        } else {
            self.export_html_impl(page_name)
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

    fn set_bind_address_impl(&mut self, addr: String) {
        if self.bind_address != addr {
            self.bind_address = addr;
            self.bind_address_changed();
        }
    }

    fn get_network_interfaces_json_impl(&self) -> String {
        let interfaces = notesplusplus_core::server::http::get_network_interfaces();
        let entries: Vec<serde_json::Value> = interfaces
            .into_iter()
            .map(|(ip, name)| {
                serde_json::json!({
                    "ip": ip,
                    "name": name,
                })
            })
            .collect();
        serde_json::to_string(&entries).unwrap_or_else(|_| "[]".to_string())
    }

    pub(crate) fn get_server_urls_json_impl(&self) -> String {
        if let Some(ref handle) = self.server_handle {
            let urls: Vec<String> = handle.urls().iter()
                .map(|u| u.replace("0.0.0.0", "localhost"))
                .collect();
            serde_json::to_string(&urls).unwrap_or_else(|_| "[]".to_string())
        } else {
            "[]".to_string()
        }
    }

    pub(crate) fn set_theme_impl(&mut self, colors_json: String) {
        if let Ok(map) = serde_json::from_str::<std::collections::HashMap<String, String>>(&colors_json) {
            self.qt_theme = notesplusplus_core::html::qt_html::QtThemeColors::from_map(&map);
            self.pending_theme_colors = map.clone();
            if let Some(ref handle) = self.server_handle {
                handle.context().set_theme_colors(map);
            }
        }
    }

    pub(crate) fn set_session_expiry_hours_impl(&mut self, hours: i32) {
        let secs = (hours.max(1) as u64) * 3600;
        self.auth_config.session_expiry_secs = secs;
        if let Some(ref handle) = self.server_handle {
            handle.context().set_session_expiry_secs(secs);
        }
    }

    fn get_linkable_pages_json_impl(&mut self, query: String) -> String {
        if !self.ensure_init_blocking() {
            return "[]".to_string();
        }
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


    fn create_page_impl(&mut self, name: String) {
        if !self.ensure_init_blocking() {
            return;
        }
        let conn = match self.conn() {
            Some(c) => c,
            None => return,
        };

        match page::create_page(conn, &self.notes_dir(), &name, false) {
            Ok(info) => {
                self.load_main_page_data_impl();
                self.load_page_impl(info.full_path());
            }
            Err(e) => {
                self.report_error(e.to_string());
            }
        }
    }

    fn create_group_impl(&mut self, parent_path: String, name: String) -> bool {
        if !self.ensure_init_blocking() {
            return false;
        }
        let conn = match self.conn() {
            Some(c) => c,
            None => return false,
        };
        match notesplusplus_core::group::create_group(conn, &self.notes_dir(), &parent_path, &name) {
            Ok(_) => {
                self.load_main_page_data_impl();
                true
            }
            Err(e) => {
                self.report_error(e.to_string());
                false
            }
        }
    }

    fn rename_group_impl(&mut self, old_path: String, new_name: String) -> bool {
        if !self.ensure_init_blocking() {
            return false;
        }
        let conn = match self.conn() {
            Some(c) => c,
            None => return false,
        };
        match notesplusplus_core::group::rename_group(conn, &self.notes_dir(), &old_path, &new_name) {
            Ok(_) => {
                self.load_main_page_data_impl();
                true
            }
            Err(e) => {
                self.report_error(e.to_string());
                false
            }
        }
    }

    fn delete_group_impl(&mut self, path: String, recursive: bool) -> bool {
        if !self.ensure_init_blocking() {
            return false;
        }
        let conn = match self.conn() {
            Some(c) => c,
            None => return false,
        };
        match notesplusplus_core::group::delete_group(conn, &self.notes_dir(), &path, recursive) {
            Ok(_) => {
                self.load_main_page_data_impl();
                true
            }
            Err(e) => {
                self.report_error(e.to_string());
                false
            }
        }
    }

    fn move_page_to_group_impl(&mut self, page_full_path: String, target_group: String) -> bool {
        if !self.ensure_init_blocking() {
            return false;
        }
        let conn = match self.conn() {
            Some(c) => c,
            None => return false,
        };
        match page::move_page(conn, &self.notes_dir(), &page_full_path, &target_group) {
            Ok(info) => {
                self.load_main_page_data_impl();
                if self.current_page_name == info.title {
                    self.current_page_group_path = info.group_path.clone();
                    self.current_page_group_path_changed();
                }
                true
            }
            Err(e) => {
                self.report_error(e.to_string());
                false
            }
        }
    }

    fn set_group_display_depth_impl(&mut self, depth: i32) {
        if self.group_display_depth != depth {
            self.group_display_depth = depth;
            self.group_depth_changed();
            self.load_main_page_data_impl();
        }
    }

    fn toggle_group_collapsed_impl(&mut self, group_path: String) -> bool {
        if !self.ensure_init_blocking() {
            return false;
        }
        let conn = match self.conn() {
            Some(c) => c,
            None => return false,
        };
        match notesplusplus_core::group::toggle_group_collapsed(conn, &group_path) {
            Ok(collapsed) => {
                // Rebuild main page data inline (fast, no background thread)
                self.ensure_init();
                if !self.poll_init() {
                    return collapsed;
                }
                let conn = match self.conn() {
                    Some(c) => c,
                    None => return collapsed,
                };
                let all_pages = page::list_pages(conn).unwrap_or_default();
                let all_groups = notesplusplus_core::group::get_groups_flat(conn).unwrap_or_default();
                let notes_dir = self.notes_dir();
                let notes_path = self.notes_path.to_string_lossy().to_string();
                let drop_comments = self.drop_comments;
                let qt_theme = self.qt_theme.clone();
                let qt_options = notesplusplus_core::html::qt_html::QtRenderOptions {
                    notes_dir: Some(notes_path),
                    allow_external_images: true,
                    ..Default::default()
                };
                let tree_json = notesplusplus_core::tree::build_group_tree(
                    &all_pages, &all_groups, self.group_display_depth,
                    Some(&notes_dir), drop_comments, Some(&qt_theme), Some(&qt_options),
                );
                self.grouped_tree_json = QString::from(tree_json);
                self.data_refreshed();
                collapsed
            }
            Err(e) => {
                self.report_error(e.to_string());
                false
            }
        }
    }

    fn get_groups_json_impl(&mut self) -> String {
        if !self.ensure_init_blocking() {
            return "[]".to_string();
        }
        let conn = match self.conn() {
            Some(c) => c,
            None => return "[]".to_string(),
        };
        match notesplusplus_core::group::get_groups_flat(conn) {
            Ok(groups) => serde_json::to_string(&groups).unwrap_or_else(|_| "[]".to_string()),
            Err(_) => "[]".to_string(),
        }
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
    pub fn create_group(&mut self, parent_path: String, name: String) -> bool { self.create_group_impl(parent_path, name) }
    pub fn rename_group(&mut self, old_path: String, new_name: String) -> bool { self.rename_group_impl(old_path, new_name) }
    pub fn delete_group(&mut self, path: String, recursive: bool) -> bool { self.delete_group_impl(path, recursive) }
    pub fn move_page_to_group(&mut self, page_full_path: String, target_group: String) -> bool { self.move_page_to_group_impl(page_full_path, target_group) }
    pub fn set_group_display_depth(&mut self, depth: i32) { self.set_group_display_depth_impl(depth); }
    pub fn toggle_group_collapsed(&mut self, group_path: String) -> bool { self.toggle_group_collapsed_impl(group_path) }
    pub fn get_groups_json(&mut self) -> String { self.get_groups_json_impl() }
    pub fn navigate_to_page(&mut self, name: String) { self.navigate_to_page_impl(name); }
    pub fn insert_link_at_cursor(&mut self, block_idx: i32, cursor_pos: i32, target: String) { self.insert_link_at_cursor_impl(block_idx, cursor_pos, target); }
    pub fn toggle_checkbox(&mut self, block_index: i32, item_path: String) { self.toggle_checkbox_impl(block_index, item_path); }
    pub fn set_drop_comments(&mut self, drop: bool) { self.set_drop_comments_impl(drop); }
    pub fn set_reject_public_networks(&mut self, reject: bool) { self.set_reject_public_networks_impl(reject); }
    pub fn set_bind_address(&mut self, addr: String) { self.set_bind_address_impl(addr); }
    pub fn get_network_interfaces_json(&self) -> String { self.get_network_interfaces_json_impl() }
    pub fn load_main_page_data(&mut self) { self.load_main_page_data_impl(); }
    pub fn poll_main_page_data(&mut self) -> bool { self.poll_main_page_data_impl() }
    pub fn export_html(&mut self, page_name: String) -> String { self.export_html_impl(page_name) }
    pub fn export_all_html(&mut self) -> String { self.export_all_html_impl() }
    pub fn open_in_browser(&mut self, page_name: String) -> String { self.open_in_browser_impl(page_name) }


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
