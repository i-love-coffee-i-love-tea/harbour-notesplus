use super::FishdocBridge;

impl FishdocBridge {
    pub fn poll_results(&mut self) -> bool {
        // Check page load results
        let mut pending = match self.pending.lock() {
            Ok(p) => p,
            Err(_) => return false,
        };

        if let Some(result) = pending.take() {
            self.is_loading = false;
            self.loading_changed();

            if let Some(error) = result.error {
                self.error_message = error;
                self.error_occurred(self.error_message.clone());
                return true;
            }

            if let Some(blocks) = result.blocks {
                self.current_blocks = Self::blocks_to_qvariantlist(&blocks);
                self.current_blocks_data = blocks;
                self.blocks_version += 1;
                self.page_changed();
            }

            return true;
        }
        drop(pending);

        // Check PDF export results
        let mut pdf_pending = match self.pdf_pending.lock() {
            Ok(p) => p,
            Err(_) => return false,
        };

        if let Some(result) = pdf_pending.take() {
            self.is_loading = false;
            self.loading_changed();

            if let Some(error) = result.error {
                self.error_message = error;
                self.error_occurred(self.error_message.clone());
                return true;
            }

            if let Some(path) = result.path {
                self.pdf_exported(path);
            }

            return true;
        }

        false
    }
}
