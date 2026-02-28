#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BufferState {
    pub confirmed_text: String,
    pub live_segment: String,
}

impl BufferState {
    pub fn append_live(&mut self, text: &str) {
        append_with_boundary_spacing(&mut self.live_segment, text);
    }

    pub fn commit_live(&mut self) {
        append_with_boundary_spacing(&mut self.confirmed_text, &self.live_segment);
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
        append_with_boundary_spacing(&mut full, &self.live_segment);
        full
    }

    pub fn replace_confirmed(&mut self, text: String) {
        self.confirmed_text = text;
    }
}

fn append_with_boundary_spacing(target: &mut String, text: &str) {
    if text.is_empty() {
        return;
    }

    let needs_space = target
        .chars()
        .last()
        .zip(text.chars().next())
        .map(|(left, right)| should_insert_space_between(left, right))
        .unwrap_or(false);

    if needs_space {
        target.push(' ');
    }
    target.push_str(text);
}

fn should_insert_space_between(left: char, right: char) -> bool {
    if left.is_whitespace() || right.is_whitespace() {
        return false;
    }

    if left.is_ascii_alphanumeric() && right.is_ascii_alphanumeric() {
        return true;
    }

    ".!?,:;)]}".contains(left) && right.is_ascii_alphanumeric()
}

#[cfg(test)]
mod tests {
    use super::BufferState;

    #[test]
    fn append_live_inserts_boundary_space_between_ascii_words() {
        let mut buffer = BufferState::default();
        buffer.append_live("hello");
        buffer.append_live("world");

        assert_eq!(buffer.live_segment, "hello world");
    }

    #[test]
    fn append_live_does_not_duplicate_space_when_chunk_already_has_it() {
        let mut buffer = BufferState::default();
        buffer.append_live("hello");
        buffer.append_live(" world");

        assert_eq!(buffer.live_segment, "hello world");
    }

    #[test]
    fn commit_live_inserts_boundary_space_from_confirmed_to_live() {
        let mut buffer = BufferState {
            confirmed_text: "hello".to_owned(),
            live_segment: "world".to_owned(),
        };

        buffer.commit_live();

        assert_eq!(buffer.confirmed_text, "hello world");
    }

    #[test]
    fn full_text_inserts_boundary_space_between_confirmed_and_live() {
        let buffer = BufferState {
            confirmed_text: "hello".to_owned(),
            live_segment: "world".to_owned(),
        };

        assert_eq!(buffer.full_text(), "hello world");
    }

    #[test]
    fn punctuation_chunk_does_not_get_space_before_it() {
        let mut buffer = BufferState::default();
        buffer.append_live("hello");
        buffer.append_live(",");

        assert_eq!(buffer.live_segment, "hello,");
    }
}
