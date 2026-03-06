//! Call and value record data structures for calls.json output.
//!
//! These are the NEW structures used by the ExpressionTracker during indexing.
//! They replace the old `output::calls` structs with richer typed enums.

use serde::{Deserialize, Serialize};

/// The kind of a call site.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CallKind {
    MethodCall,
    NullsafeMethodCall,
    StaticCall,
    FuncCall,
    New,
}

/// The kind of a value access site.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValueKind {
    PropertyRead,
    PropertyWrite,
    StaticPropertyRead,
    StaticPropertyWrite,
    ClassConstRead,
    ArrayDimRead,
    ArrayDimWrite,
}

/// A call argument value, representing the expression passed to a call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ArgumentValue {
    StringLiteral { value: String },
    IntLiteral { value: i64 },
    FloatLiteral { value: f64 },
    BoolLiteral { value: bool },
    NullLiteral,
    Variable { name: String },
    ClassConst { class: String, name: String },
    StaticPropertyFetch { class: String, name: String },
    PropertyFetch { object_type: Option<String>, name: String },
    MethodCall { object_type: Option<String>, method: String },
    StaticCall { class: String, method: String },
    FuncCall { function: String },
    New { class: String },
    ArrayLiteral { elements: Vec<ArgumentValue> },
    Unknown,
}

/// A method/function call site record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallRecord {
    pub caller: String,
    pub callee: String,
    pub kind: CallKind,
    pub file: String,
    pub line: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub arguments: Vec<ArgumentValue>,
}

/// A value/property access record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueRecord {
    pub source: String,
    pub target: String,
    pub kind: ValueKind,
    pub file: String,
    pub line: u32,
}

/// Root structure for calls.json output.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CallsOutput {
    pub calls: Vec<CallRecord>,
    pub values: Vec<ValueRecord>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_call_kind_serialization() {
        assert_eq!(
            serde_json::to_string(&CallKind::MethodCall).unwrap(),
            "\"method_call\""
        );
        assert_eq!(
            serde_json::to_string(&CallKind::NullsafeMethodCall).unwrap(),
            "\"nullsafe_method_call\""
        );
        assert_eq!(
            serde_json::to_string(&CallKind::StaticCall).unwrap(),
            "\"static_call\""
        );
        assert_eq!(
            serde_json::to_string(&CallKind::FuncCall).unwrap(),
            "\"func_call\""
        );
        assert_eq!(
            serde_json::to_string(&CallKind::New).unwrap(),
            "\"new\""
        );
    }

    #[test]
    fn test_call_kind_deserialization() {
        let kind: CallKind = serde_json::from_str("\"method_call\"").unwrap();
        assert_eq!(kind, CallKind::MethodCall);
        let kind: CallKind = serde_json::from_str("\"static_call\"").unwrap();
        assert_eq!(kind, CallKind::StaticCall);
    }

    #[test]
    fn test_value_kind_serialization() {
        assert_eq!(
            serde_json::to_string(&ValueKind::PropertyRead).unwrap(),
            "\"property_read\""
        );
        assert_eq!(
            serde_json::to_string(&ValueKind::StaticPropertyWrite).unwrap(),
            "\"static_property_write\""
        );
        assert_eq!(
            serde_json::to_string(&ValueKind::ClassConstRead).unwrap(),
            "\"class_const_read\""
        );
    }

    #[test]
    fn test_call_record_serialization() {
        let record = CallRecord {
            caller: "App\\Foo::bar".to_string(),
            callee: "App\\Baz::qux".to_string(),
            kind: CallKind::MethodCall,
            file: "src/Foo.php".to_string(),
            line: 10,
            arguments: vec![],
        };
        let json = serde_json::to_value(&record).unwrap();
        assert_eq!(json["caller"], "App\\Foo::bar");
        assert_eq!(json["callee"], "App\\Baz::qux");
        assert_eq!(json["kind"], "method_call");
        assert_eq!(json["file"], "src/Foo.php");
        assert_eq!(json["line"], 10);
        // arguments should be omitted when empty
        assert!(json.get("arguments").is_none());
    }

    #[test]
    fn test_call_record_with_arguments() {
        let record = CallRecord {
            caller: "App\\Foo::bar".to_string(),
            callee: "App\\Baz::qux".to_string(),
            kind: CallKind::StaticCall,
            file: "src/Foo.php".to_string(),
            line: 15,
            arguments: vec![
                ArgumentValue::StringLiteral {
                    value: "hello".to_string(),
                },
                ArgumentValue::IntLiteral { value: 42 },
                ArgumentValue::BoolLiteral { value: true },
            ],
        };
        let json = serde_json::to_value(&record).unwrap();
        let args = json["arguments"].as_array().unwrap();
        assert_eq!(args.len(), 3);
        assert_eq!(args[0]["kind"], "string_literal");
        assert_eq!(args[0]["value"], "hello");
        assert_eq!(args[1]["kind"], "int_literal");
        assert_eq!(args[1]["value"], 42);
        assert_eq!(args[2]["kind"], "bool_literal");
        assert_eq!(args[2]["value"], true);
    }

    #[test]
    fn test_argument_value_null_literal() {
        let val = ArgumentValue::NullLiteral;
        let json = serde_json::to_value(&val).unwrap();
        assert_eq!(json["kind"], "null_literal");
    }

    #[test]
    fn test_argument_value_unknown() {
        let val = ArgumentValue::Unknown;
        let json = serde_json::to_value(&val).unwrap();
        assert_eq!(json["kind"], "unknown");
    }

    #[test]
    fn test_argument_value_variable() {
        let val = ArgumentValue::Variable {
            name: "$user".to_string(),
        };
        let json = serde_json::to_value(&val).unwrap();
        assert_eq!(json["kind"], "variable");
        assert_eq!(json["name"], "$user");
    }

    #[test]
    fn test_argument_value_array_literal() {
        let val = ArgumentValue::ArrayLiteral {
            elements: vec![
                ArgumentValue::IntLiteral { value: 1 },
                ArgumentValue::StringLiteral {
                    value: "two".to_string(),
                },
            ],
        };
        let json = serde_json::to_value(&val).unwrap();
        assert_eq!(json["kind"], "array_literal");
        assert_eq!(json["elements"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_value_record_serialization() {
        let record = ValueRecord {
            source: "App\\Foo::bar".to_string(),
            target: "App\\Foo#$name.".to_string(),
            kind: ValueKind::PropertyRead,
            file: "src/Foo.php".to_string(),
            line: 20,
        };
        let json = serde_json::to_value(&record).unwrap();
        assert_eq!(json["source"], "App\\Foo::bar");
        assert_eq!(json["target"], "App\\Foo#$name.");
        assert_eq!(json["kind"], "property_read");
        assert_eq!(json["line"], 20);
    }

    #[test]
    fn test_calls_output_default() {
        let output = CallsOutput::default();
        assert!(output.calls.is_empty());
        assert!(output.values.is_empty());
    }

    #[test]
    fn test_calls_output_serialization() {
        let output = CallsOutput {
            calls: vec![CallRecord {
                caller: "main".to_string(),
                callee: "helper".to_string(),
                kind: CallKind::FuncCall,
                file: "index.php".to_string(),
                line: 5,
                arguments: vec![],
            }],
            values: vec![],
        };
        let json = serde_json::to_value(&output).unwrap();
        assert_eq!(json["calls"].as_array().unwrap().len(), 1);
        assert_eq!(json["values"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn test_call_record_roundtrip() {
        let record = CallRecord {
            caller: "App\\Ctrl::handle".to_string(),
            callee: "App\\Svc::process".to_string(),
            kind: CallKind::MethodCall,
            file: "src/Ctrl.php".to_string(),
            line: 42,
            arguments: vec![ArgumentValue::Variable {
                name: "$req".to_string(),
            }],
        };
        let json_str = serde_json::to_string(&record).unwrap();
        let deserialized: CallRecord = serde_json::from_str(&json_str).unwrap();
        assert_eq!(deserialized.caller, record.caller);
        assert_eq!(deserialized.callee, record.callee);
        assert_eq!(deserialized.kind, record.kind);
        assert_eq!(deserialized.line, record.line);
        assert_eq!(deserialized.arguments.len(), 1);
    }

    #[test]
    fn test_argument_class_const() {
        let val = ArgumentValue::ClassConst {
            class: "App\\Status".to_string(),
            name: "ACTIVE".to_string(),
        };
        let json = serde_json::to_value(&val).unwrap();
        assert_eq!(json["kind"], "class_const");
        assert_eq!(json["class"], "App\\Status");
        assert_eq!(json["name"], "ACTIVE");
    }

    #[test]
    fn test_argument_float_literal() {
        let val = ArgumentValue::FloatLiteral { value: 3.14 };
        let json = serde_json::to_value(&val).unwrap();
        assert_eq!(json["kind"], "float_literal");
        assert_eq!(json["value"], 3.14);
    }
}
