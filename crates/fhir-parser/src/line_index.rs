/// Index for converting byte offsets to 1-indexed (line, column) pairs.
pub(crate) struct LineIndex {
    /// Byte offset of the start of each line (0-indexed lines).
    line_starts: Vec<u32>,
}

impl LineIndex {
    pub(crate) fn new(source: &str) -> Self {
        let mut line_starts = vec![0u32];
        for (i, b) in source.bytes().enumerate() {
            if b == b'\n' {
                line_starts.push((i + 1) as u32);
            }
        }
        Self { line_starts }
    }

    /// Returns (line, col) both 1-indexed for the given byte offset.
    pub(crate) fn location(&self, offset: u32) -> (u32, u32) {
        let line_idx = self
            .line_starts
            .partition_point(|&s| s <= offset)
            .saturating_sub(1);
        let col = offset - self.line_starts[line_idx];
        (line_idx as u32 + 1, col + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::LineIndex;

    #[test]
    fn single_line() {
        let idx = LineIndex::new("hello world");
        assert_eq!(idx.location(0), (1, 1));
        assert_eq!(idx.location(5), (1, 6));
    }

    #[test]
    fn multiline() {
        let idx = LineIndex::new("ab\ncd\nef");
        assert_eq!(idx.location(0), (1, 1));
        assert_eq!(idx.location(2), (1, 3)); // the '\n'
        assert_eq!(idx.location(3), (2, 1));
        assert_eq!(idx.location(4), (2, 2));
        assert_eq!(idx.location(6), (3, 1));
    }
}
