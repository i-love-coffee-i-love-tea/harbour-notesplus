use super::NotesBridge;

impl NotesBridge {
    pub fn poll_results(&mut self) -> bool {
        // Check page load results
        let mut pending = match self.pending.lock() {
            Ok(p) => p,
            Err(_) => return false,
        };

        let taken = pending.take();
        drop(pending);

        if let Some(result) = taken {
            self.is_loading = false;
            self.loading_changed();

            if let Some(error) = result.error {
                self.report_error(error);
                return true;
            }

            if let Some(blocks) = result.blocks {
                let options = notesplusplus_core::html::qt_html::QtRenderOptions {
                    notes_dir: Some(self.notes_path.to_string_lossy().to_string()),
                    allow_external_images: true,
                    search_terms: Vec::new(),
                };
                self.current_blocks = Self::blocks_to_qvariantlist_with_html(&blocks, &self.qt_theme, &options);
                self.current_blocks_data = blocks;
                self.blocks_version += 1;
                self.page_changed();
            }

            return true;
        }

        false
    }
}
