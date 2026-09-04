use qmetaobject::*;

use fishdoc_core::search as search_mod;

use super::FishdocBridge;

impl FishdocBridge {
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
                    let mut map = QVariantMap::default();
                    map.insert("name".into(), QString::from(r.page.title.clone()).into());
                    map.insert("filename".into(), QString::from(r.page.filename.clone()).into());
                    map.insert("snippet".into(), QString::from(r.snippet.clone()).into());
                    list.push(map.into());
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
}
