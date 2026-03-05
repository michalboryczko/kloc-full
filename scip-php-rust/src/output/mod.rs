pub mod scip;
pub mod calls;

use anyhow::Result;
use std::path::Path;

pub use calls::CallsIndex;
pub use scip::ScipIndex;

/// Writes the unified JSON output to disk.
pub struct UnifiedJsonWriter {
    pub index: ScipIndex,
    pub calls: CallsIndex,
}

impl UnifiedJsonWriter {
    pub fn new() -> Self {
        UnifiedJsonWriter {
            index: ScipIndex::default(),
            calls: CallsIndex::default(),
        }
    }

    pub fn write(&self, output_dir: &Path) -> Result<()> {
        std::fs::create_dir_all(output_dir)?;

        let index_path = output_dir.join("index.json");
        let index_json = serde_json::to_string_pretty(&self.index)?;
        std::fs::write(&index_path, index_json)?;

        let calls_path = output_dir.join("calls.json");
        let calls_json = serde_json::to_string_pretty(&self.calls)?;
        std::fs::write(&calls_path, calls_json)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scip::{Occurrence, symbol_roles};

    #[test]
    fn test_empty_index_json_shape() {
        let writer = UnifiedJsonWriter::new();
        let json = serde_json::to_value(&writer.index).unwrap();

        // Verify top-level keys exist
        assert!(json.get("metadata").is_some());
        assert!(json.get("documents").is_some());

        // documents should be empty array
        assert_eq!(json["documents"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn test_occurrence_serialization() {
        let occ = Occurrence {
            range: vec![0, 5, 0, 10],
            symbol: "scip-php composer foo/bar 1.0.0 Foo/Bar#".to_string(),
            symbol_roles: symbol_roles::DEFINITION,
            override_documentation: vec![],
            diagnostics: vec![],
            enclosing_range: vec![],
        };
        let json = serde_json::to_string(&occ).unwrap();
        // Verify range is array of 4 numbers
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["range"].as_array().unwrap().len(), 4);
        assert_eq!(parsed["symbol_roles"].as_u64().unwrap(), 1);
    }

    #[test]
    fn test_calls_json_shape() {
        let calls = CallsIndex::default();
        let json = serde_json::to_value(&calls).unwrap();
        assert!(json.get("calls").is_some());
        assert!(json.get("values").is_some());
    }
}
