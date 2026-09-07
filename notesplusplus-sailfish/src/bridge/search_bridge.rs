use qmetaobject::*;

use notesplusplus_core::page;
use notesplusplus_core::search as search_mod;

use super::NotesBridge;

impl NotesBridge {
    pub fn do_search(&mut self, query: String) {
        self.ensure_init();
        self.search_query = query.clone();

        if query.trim().is_empty() {
            self.search_results = QVariantList::default();
            self.search_results_changed();
            return;
        }

        let conn = match self.conn() {
            Some(c) => c,
            None => return,
        };

        match search_mod::search_pages(conn, &query) {
            Ok(results) => {
                let mut list = QVariantList::default();
                for r in &results {
                    let preview_values = page::get_page_preview_values_with_options(&self.notes_path, &r.page.filename, 8, self.drop_comments);
                    let preview_json_str = serde_json::to_string(&preview_values).unwrap_or_else(|_| "[]".to_string());

                    let mut map = serde_json::Map::new();
                    map.insert("name".into(), serde_json::Value::String(r.page.title.clone()));
                    map.insert("filename".into(), serde_json::Value::String(r.page.filename.clone()));
                    map.insert("snippet".into(), serde_json::Value::String(r.snippet.clone()));
                    map.insert("query".into(), serde_json::Value::String(query.clone()));
                    map.insert("created_at".into(), serde_json::Value::String(r.page.created_at.clone()));
                    map.insert("updated_at".into(), serde_json::Value::String(r.page.updated_at.clone()));
                    map.insert("block_count".into(), serde_json::Value::Number(r.page.block_count.into()));
                    map.insert("preview_blocks".into(), serde_json::Value::Array(preview_values));
                    map.insert("preview_blocks_json".into(), serde_json::Value::String(preview_json_str));

                    let json_str = serde_json::to_string(&serde_json::Value::Object(map)).unwrap_or_default();
                    list.push(QString::from(json_str).into());
                }
                self.search_results = list;
                self.search_results_changed();
            }
            Err(e) => {
                self.error_message = format!("Search error: {}", e);
                self.error_occurred(self.error_message.clone());
            }
        }
    }

    pub fn search(&mut self, query: String) {
        self.do_search(query);
    }
}
