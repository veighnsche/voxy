#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BufferState {
    pub confirmed_text: String,
    pub live_segment: String,
}

impl BufferState {
    pub fn append_live(&mut self, text: &str) {
        self.live_segment.push_str(text);
    }

    pub fn commit_live(&mut self) {
        self.confirmed_text.push_str(&self.live_segment);
        self.live_segment.clear();
    }

    pub fn clear_live(&mut self) {
        self.live_segment.clear();
    }

    pub fn reset_all(&mut self) {
        self.confirmed_text.clear();
        self.live_segment.clear();
    }

    pub fn full_text(&self) -> String {
        let mut full = String::with_capacity(self.confirmed_text.len() + self.live_segment.len());
        full.push_str(&self.confirmed_text);
        full.push_str(&self.live_segment);
        full
    }

    pub fn replace_confirmed(&mut self, text: String) {
        self.confirmed_text = text;
    }
}
