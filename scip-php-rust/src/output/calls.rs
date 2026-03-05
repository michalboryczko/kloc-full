use serde::{Deserialize, Serialize};

/// Root structure for calls.json
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CallsIndex {
    pub calls: Vec<CallRecord>,
    pub values: Vec<ValueRecord>,
}

/// A method/function call site record
#[derive(Debug, Serialize, Deserialize)]
pub struct CallRecord {
    /// The caller symbol (method or function making the call)
    pub caller: String,
    /// The callee symbol being called
    pub callee: String,
    /// File path (relative to project root)
    pub file: String,
    /// Source range [start_line, start_char, end_line, end_char]
    pub range: Vec<u32>,
    /// Call kind: "method_call", "static_call", "function_call", "new"
    pub kind: String,
}

/// A value/type annotation record
#[derive(Debug, Serialize, Deserialize)]
pub struct ValueRecord {
    /// Symbol being annotated
    pub symbol: String,
    /// Resolved type string (FQN)
    pub value_type: String,
    /// File path (relative to project root)
    pub file: String,
    /// Source range
    pub range: Vec<u32>,
}
