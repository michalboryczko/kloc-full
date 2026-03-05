use serde::{Deserialize, Serialize};

/// Root SCIP index document
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ScipIndex {
    pub metadata: Metadata,
    pub documents: Vec<Document>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub external_symbols: Vec<SymbolInformation>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Metadata {
    pub version: String,
    pub tool_info: ToolInfo,
    pub project_root: String,
    pub text_document_encoding: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ToolInfo {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub arguments: Vec<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Document {
    /// Relative path from project root
    pub relative_path: String,
    /// MIME type — always "application/x-php" for PHP files
    pub language: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub occurrences: Vec<Occurrence>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub symbols: Vec<SymbolInformation>,
}

/// A SCIP occurrence — a location in source mapped to a symbol.
#[derive(Debug, Serialize, Deserialize)]
pub struct Occurrence {
    /// [start_line, start_char, end_line, end_char] (0-indexed)
    pub range: Vec<u32>,
    pub symbol: String,
    /// Bit flags: 1 = definition, 2 = reference
    pub symbol_roles: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub override_documentation: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub enclosing_range: Vec<u32>,
}

/// Symbol roles constants
pub mod symbol_roles {
    pub const DEFINITION: u32 = 1;
    pub const REFERENCE: u32 = 2;
    pub const FORWARD_DEFINITION: u32 = 4;
    pub const READ_ACCESS: u32 = 8;
    pub const WRITE_ACCESS: u32 = 16;
    pub const TAG_DEFINITION: u32 = 32;
    pub const TAG_REFERENCE: u32 = 64;
    pub const IMPLEMENTATION: u32 = 128;
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SymbolInformation {
    pub symbol: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub documentation: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relationships: Vec<Relationship>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Relationship {
    pub symbol: String,
    #[serde(default)]
    pub is_reference: bool,
    #[serde(default)]
    pub is_implementation: bool,
    #[serde(default)]
    pub is_type_definition: bool,
    #[serde(default)]
    pub is_definition: bool,
}
