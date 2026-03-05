//! Position and range types for mapping CST node locations to SCIP format.

use tree_sitter::Node;

/// A source location (0-indexed line and column).
///
/// tree-sitter uses 0-indexed rows and columns.
/// SCIP also uses 0-indexed. No conversion needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub line: u32,
    pub col: u32,
}

impl Position {
    pub fn new(line: u32, col: u32) -> Self {
        Position { line, col }
    }

    /// Convert from tree-sitter Point.
    pub fn from_ts_point(point: tree_sitter::Point) -> Self {
        Position {
            line: point.row as u32,
            col: point.column as u32,
        }
    }
}

/// A source range (start..end positions, both 0-indexed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

impl Range {
    pub fn from_node(node: Node<'_>) -> Self {
        Range {
            start: Position::from_ts_point(node.start_position()),
            end: Position::from_ts_point(node.end_position()),
        }
    }

    /// Encode as SCIP range array [start_line, start_char, end_line, end_char].
    ///
    /// SCIP uses a compact encoding: if start_line == end_line, the array may
    /// be [start_line, start_char, end_char] (3 elements). However, PHP scip-php
    /// always emits 4 elements. Match that behavior.
    pub fn to_scip_vec(&self) -> Vec<u32> {
        vec![self.start.line, self.start.col, self.end.line, self.end.col]
    }
}

/// A byte range in the source text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ByteRange {
    pub start: usize,
    pub end: usize,
}

impl ByteRange {
    pub fn from_node(node: Node<'_>) -> Self {
        ByteRange {
            start: node.start_byte(),
            end: node.end_byte(),
        }
    }

    pub fn len(&self) -> usize {
        self.end - self.start
    }

    pub fn extract_text<'a>(&self, source: &'a [u8]) -> &'a str {
        unsafe { std::str::from_utf8_unchecked(&source[self.start..self.end]) }
    }
}

/// Pre-computed line start byte offsets for efficient line/column lookups.
///
/// Given a byte offset, find the line number and column with O(log n) binary search.
/// Built once per file, used throughout indexing of that file.
pub struct LineOffsetCache {
    /// Byte offset of the start of each line.
    /// line_starts[0] = 0 (start of file)
    /// line_starts[1] = byte offset of second line
    /// etc.
    line_starts: Vec<usize>,
}

impl LineOffsetCache {
    /// Build the line offset cache from source bytes.
    pub fn new(source: &[u8]) -> Self {
        let mut line_starts = vec![0usize];
        for (i, &byte) in source.iter().enumerate() {
            if byte == b'\n' {
                line_starts.push(i + 1);
            }
        }
        LineOffsetCache { line_starts }
    }

    /// Convert a byte offset to (line, column), both 0-indexed.
    pub fn byte_to_position(&self, byte_offset: usize) -> Position {
        // Binary search for the line
        let line = match self.line_starts.binary_search(&byte_offset) {
            Ok(exact) => exact,
            Err(insertion_point) => insertion_point - 1,
        };
        let col = byte_offset - self.line_starts[line];
        Position {
            line: line as u32,
            col: col as u32,
        }
    }

    /// Total number of lines.
    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }
}

/// Extension trait to easily get Range from a tree-sitter node.
pub trait NodeRange {
    fn range(&self) -> Range;
    fn byte_range(&self) -> ByteRange;
    fn scip_range(&self) -> Vec<u32>;
}

impl NodeRange for tree_sitter::Node<'_> {
    fn range(&self) -> Range {
        Range::from_node(*self)
    }

    fn byte_range(&self) -> ByteRange {
        ByteRange::from_node(*self)
    }

    fn scip_range(&self) -> Vec<u32> {
        Range::from_node(*self).to_scip_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_php(source: &str) -> (tree_sitter::Tree, Vec<u8>) {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_php::LANGUAGE_PHP.into())
            .unwrap();
        let bytes = source.as_bytes().to_vec();
        let tree = parser.parse(&bytes, None).unwrap();
        (tree, bytes)
    }

    #[test]
    fn test_position_is_zero_indexed() {
        // "<?php\nclass Foo {}" — "Foo" is on line 1 (0-indexed), col 6
        let (tree, _) = parse_php("<?php\nclass Foo {}");
        let root = tree.root_node();
        // program -> [php_tag, class_declaration]
        // class_declaration is named_child(1) (after php_tag)
        let class = root.named_child(1).unwrap();
        assert_eq!(class.kind(), "class_declaration");
        // The class name node has kind "name" and is named_child(0) of class_declaration
        let name = class.child_by_field_name("name").unwrap();
        let pos = Position::from_ts_point(name.start_position());
        assert_eq!(pos.line, 1, "line should be 0-indexed: line 1 is the second line");
        assert_eq!(pos.col, 6, "col should be 0-indexed: 'class ' is 6 chars");
    }

    #[test]
    fn test_range_to_scip_vec() {
        let range = Range {
            start: Position::new(0, 5),
            end: Position::new(0, 8),
        };
        assert_eq!(range.to_scip_vec(), vec![0, 5, 0, 8]);
    }

    #[test]
    fn test_range_to_scip_vec_multiline() {
        let range = Range {
            start: Position::new(2, 0),
            end: Position::new(5, 1),
        };
        assert_eq!(range.to_scip_vec(), vec![2, 0, 5, 1]);
    }

    #[test]
    fn test_line_offset_cache_single_line() {
        let source = b"hello world";
        let cache = LineOffsetCache::new(source);
        assert_eq!(cache.line_count(), 1);
        let pos = cache.byte_to_position(6);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.col, 6);
    }

    #[test]
    fn test_line_offset_cache_multiline() {
        // "abc\ndef\nghi"
        // line 0: bytes 0-3 (abc\n)
        // line 1: bytes 4-7 (def\n)
        // line 2: bytes 8-10 (ghi)
        let source = b"abc\ndef\nghi";
        let cache = LineOffsetCache::new(source);
        assert_eq!(cache.line_count(), 3);

        let pos_d = cache.byte_to_position(4); // 'd' in "def"
        assert_eq!(pos_d.line, 1);
        assert_eq!(pos_d.col, 0);

        let pos_g = cache.byte_to_position(8); // 'g' in "ghi"
        assert_eq!(pos_g.line, 2);
        assert_eq!(pos_g.col, 0);

        let pos_i = cache.byte_to_position(10); // 'i' in "ghi"
        assert_eq!(pos_i.line, 2);
        assert_eq!(pos_i.col, 2);
    }

    #[test]
    fn test_line_offset_cache_start_of_file() {
        let source = b"abc\ndef";
        let cache = LineOffsetCache::new(source);
        let pos = cache.byte_to_position(0);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.col, 0);
    }

    #[test]
    fn test_node_range_extension_trait() {
        let (tree, _) = parse_php("<?php\nclass Foo {}");
        let root = tree.root_node();
        // class_declaration is named_child(1) (after php_tag)
        let class = root.named_child(1).unwrap();
        assert_eq!(class.kind(), "class_declaration");
        let scip = class.scip_range();
        assert_eq!(scip.len(), 4);
        assert_eq!(scip[0], 1); // line 1 (0-indexed)
    }
}
