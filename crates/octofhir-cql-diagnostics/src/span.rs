//! Source span and location tracking for CQL parsing

use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::Range;

/// A span in the source code, represented as a byte range
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Span {
    /// Start byte offset (inclusive)
    pub start: usize,
    /// End byte offset (exclusive)
    pub end: usize,
}

impl Span {
    /// Create a new span from start and end offsets
    #[inline]
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// Create a zero-width span at a position
    #[inline]
    pub const fn point(pos: usize) -> Self {
        Self { start: pos, end: pos }
    }

    /// Create a span covering a single byte
    #[inline]
    pub const fn single(pos: usize) -> Self {
        Self { start: pos, end: pos + 1 }
    }

    /// Get the length of the span in bytes
    #[inline]
    pub const fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    /// Check if the span is empty
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.start >= self.end
    }

    /// Merge two spans into one that covers both
    #[inline]
    pub fn merge(self, other: Self) -> Self {
        Self {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }

    /// Check if this span contains another span
    #[inline]
    pub const fn contains(&self, other: &Self) -> bool {
        self.start <= other.start && other.end <= self.end
    }

    /// Check if this span contains a position
    #[inline]
    pub const fn contains_pos(&self, pos: usize) -> bool {
        self.start <= pos && pos < self.end
    }

    /// Convert to a range
    #[inline]
    pub const fn as_range(&self) -> Range<usize> {
        self.start..self.end
    }
}

impl From<Range<usize>> for Span {
    fn from(range: Range<usize>) -> Self {
        Self::new(range.start, range.end)
    }
}

impl From<Span> for Range<usize> {
    fn from(span: Span) -> Self {
        span.start..span.end
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.start, self.end)
    }
}

/// Source location with line and column information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceLocation {
    /// Line number (1-based)
    pub line: usize,
    /// Column number (1-based)
    pub column: usize,
    /// Byte offset from start (0-based)
    pub offset: usize,
    /// Length in bytes
    pub length: usize,
}

impl SourceLocation {
    /// Create a new source location
    pub const fn new(line: usize, column: usize, offset: usize, length: usize) -> Self {
        Self {
            line,
            column,
            offset,
            length,
        }
    }

    /// Create a point location with length 1
    pub const fn point(line: usize, column: usize, offset: usize) -> Self {
        Self::new(line, column, offset, 1)
    }

    /// Create from a span and source text
    pub fn from_span(span: Span, source: &str) -> Self {
        let (line, column) = offset_to_line_col(source, span.start);
        Self {
            line,
            column,
            offset: span.start,
            length: span.len(),
        }
    }

    /// Get the span for this location
    pub const fn span(&self) -> Span {
        Span::new(self.offset, self.offset + self.length)
    }
}

impl Default for SourceLocation {
    fn default() -> Self {
        Self::new(1, 1, 0, 0)
    }
}

impl fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

/// Convert a byte offset to line and column numbers
pub fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let mut line = 1;
    let mut col = 1;

    for (i, ch) in source.char_indices() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }

    (line, col)
}

/// A node with an associated span
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spanned<T> {
    /// The inner value
    pub inner: T,
    /// The source span
    pub span: Span,
}

impl<T> Spanned<T> {
    /// Create a new spanned value
    pub const fn new(inner: T, span: Span) -> Self {
        Self { inner, span }
    }

    /// Map the inner value
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Spanned<U> {
        Spanned {
            inner: f(self.inner),
            span: self.span,
        }
    }

    /// Get a reference to the inner value
    pub const fn as_ref(&self) -> Spanned<&T> {
        Spanned {
            inner: &self.inner,
            span: self.span,
        }
    }
}

impl<T> std::ops::Deref for Spanned<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> std::ops::DerefMut for Spanned<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_merge() {
        let a = Span::new(0, 5);
        let b = Span::new(3, 10);
        assert_eq!(a.merge(b), Span::new(0, 10));
    }

    #[test]
    fn test_offset_to_line_col() {
        let source = "line1\nline2\nline3";
        assert_eq!(offset_to_line_col(source, 0), (1, 1));
        assert_eq!(offset_to_line_col(source, 5), (1, 6));
        assert_eq!(offset_to_line_col(source, 6), (2, 1));
        assert_eq!(offset_to_line_col(source, 12), (3, 1));
    }
}
