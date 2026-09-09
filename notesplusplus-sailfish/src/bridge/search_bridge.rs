use std::sync::{Arc, Mutex};
use std::thread;

use qmetaobject::*;

use notesplusplus_core::page;
use notesplusplus_core::search as search_mod;

use super::NotesBridge;

pub(super) struct SearchHit {
    title: String,
    filename: String,
    snippet: String,
    created_at: String,
    updated_at: String,
    block_count: i32,
    preview_json: String,
}

impl NotesBridge {
    pub fn do_search(&mut self, query: String) {
        self.ensure_init();
        self.search_query = query.clone();

        if query.trim().is_empty() {
            self.search_results = QVariantList::default();
            self.search_loading = false;
            self.search_results_changed();
            self.loading_changed();
            return;
        }

        let db_path = self.data_dir.join(notesplusplus_core::constants::DB_FILENAME);
        let result_slot = self.search_result_slot.clone();

        // Clear any previous pending result
        if let Ok(mut guard) = result_slot.lock() {
            *guard = None;
        }

        self.search_loading = true;
        self.loading_changed();

        thread::spawn(move || {
            let result = (|| -> Result<Vec<SearchHit>, String> {
                let conn = rusqlite::Connection::open_with_flags(
                    &db_path,
                    rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
                ).map_err(|e| format!("DB open error: {}", e))?;

                let results = search_mod::search_pages(&conn, &query)
                    .map_err(|e| format!("Search error: {}", e))?;

                let hits = results.iter().map(|r| SearchHit {
                    title: r.page.title.clone(),
                    filename: r.page.filename.clone(),
                    snippet: r.snippet.clone(),
                    created_at: r.page.created_at.clone(),
                    updated_at: r.page.updated_at.clone(),
                    block_count: r.page.block_count,
                    preview_json: "[]".to_string(),
                }).collect();
                Ok(hits)
            })();

            if let Ok(mut guard) = result_slot.lock() {
                *guard = Some(result);
            }
        });
    }

    pub fn poll_search(&mut self) -> bool {
        let has_result = if let Ok(guard) = self.search_result_slot.lock() {
            guard.is_some()
        } else {
            false
        };

        if !has_result {
            return false;
        }

        let result = if let Ok(mut guard) = self.search_result_slot.lock() {
            guard.take()
        } else {
            return false;
        };

        self.search_loading = false;
        self.loading_changed();

        if let Some(Ok(hits)) = result {
            // Store filenames for preview matching
            self.current_search_filenames = hits.iter().map(|h| h.filename.clone()).collect();

            // Deliver results immediately (without previews)
            self.emit_search_results(&hits);

            // Kick off background preview loading
            let filenames = self.current_search_filenames.clone();
            let notes_path = self.notes_path.clone();
            let drop_comments = self.drop_comments;
            let preview_slot = self.search_preview_slot.clone();

            thread::spawn(move || {
                let mut previews = Vec::new();
                for filename in &filenames {
                    let preview_values = page::get_page_preview_values_with_options(
                        &notes_path, filename, 8, drop_comments,
                    );
                    let preview_json = serde_json::to_string(&preview_values)
                        .unwrap_or_else(|_| "[]".to_string());
                    previews.push((filename.clone(), preview_json));
                }
                if let Ok(mut guard) = preview_slot.lock() {
                    *guard = Some(previews);
                }
            });
        } else if let Some(Err(e)) = result {
            self.error_message = e;
            self.error_occurred(self.error_message.clone());
        }

        true
    }

    pub fn poll_search_previews(&mut self) -> bool {
        let has_previews = if let Ok(guard) = self.search_preview_slot.lock() {
            guard.is_some()
        } else {
            false
        };

        if !has_previews {
            return false;
        }

        let previews = if let Ok(mut guard) = self.search_preview_slot.lock() {
            guard.take()
        } else {
            return false;
        };

        if let Some(previews) = previews {
            // Build a lookup from filename -> preview_json
            let preview_map: std::collections::HashMap<String, String> =
                previews.into_iter().collect();

            // Rebuild search results with previews using stored JSONs
            let mut list = QVariantList::default();
            for (idx, filename) in self.current_search_filenames.iter().enumerate() {
                if idx < self.current_search_jsons.len() {
                    let json_str = &self.current_search_jsons[idx];
                    if let Some(preview_json) = preview_map.get(filename) {
                        if let Ok(mut map) = serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(json_str) {
                            map.insert("preview_blocks_json".into(), serde_json::Value::String(preview_json.clone()));
                            let updated = serde_json::to_string(&serde_json::Value::Object(map)).unwrap_or_default();
                            list.push(QString::from(updated).into());
                        }
                    }
                }
            }
            if !list.is_empty() {
                self.search_results = list;
                self.search_results_changed();
            }
        }

        true
    }

    pub fn search(&mut self, query: String) {
        self.search_query = query.clone();
        self.search_results = QVariantList::default();
        self.search_loading = false;
        self.search_results_changed();
        self.loading_changed();
    }

    fn emit_search_results(&mut self, hits: &[SearchHit]) {
        let mut list = QVariantList::default();
        let mut jsons = Vec::new();
        for hit in hits {
            let mut map = serde_json::Map::new();
            map.insert("name".into(), serde_json::Value::String(hit.title.clone()));
            map.insert("filename".into(), serde_json::Value::String(hit.filename.clone()));
            map.insert("snippet".into(), serde_json::Value::String(hit.snippet.clone()));
            map.insert("query".into(), serde_json::Value::String(self.search_query.clone()));
            map.insert("created_at".into(), serde_json::Value::String(hit.created_at.clone()));
            map.insert("updated_at".into(), serde_json::Value::String(hit.updated_at.clone()));
            map.insert("block_count".into(), serde_json::Value::Number(hit.block_count.into()));
            map.insert("preview_blocks_json".into(), serde_json::Value::String(hit.preview_json.clone()));

            let json_str = serde_json::to_string(&serde_json::Value::Object(map)).unwrap_or_default();
            jsons.push(json_str.clone());
            list.push(QString::from(json_str).into());
        }
        self.current_search_jsons = jsons;
        self.search_results = list;
        self.search_results_changed();
    }
}
