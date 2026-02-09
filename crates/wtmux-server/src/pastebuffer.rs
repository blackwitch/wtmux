/// Paste buffer stack for copy/paste operations.
pub struct PasteBuffer {
    buffers: Vec<String>,
    max_buffers: usize,
}

impl PasteBuffer {
    pub fn new(max_buffers: usize) -> Self {
        PasteBuffer {
            buffers: Vec::new(),
            max_buffers,
        }
    }

    /// Push text onto the buffer stack.
    pub fn push(&mut self, text: String) {
        if self.buffers.len() >= self.max_buffers {
            self.buffers.remove(0);
        }
        self.buffers.push(text);
    }

    /// Get the most recent buffer content.
    pub fn top(&self) -> Option<&str> {
        self.buffers.last().map(|s| s.as_str())
    }

    /// Get a buffer by index (0 = most recent).
    pub fn get(&self, index: usize) -> Option<&str> {
        if index < self.buffers.len() {
            Some(&self.buffers[self.buffers.len() - 1 - index])
        } else {
            None
        }
    }

    /// Number of buffers.
    pub fn len(&self) -> usize {
        self.buffers.len()
    }

    /// Whether the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.buffers.is_empty()
    }
}
