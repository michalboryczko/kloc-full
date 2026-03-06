//! Symbol naming: SCIP symbol string construction.

pub mod namer;
pub mod scope;

pub use namer::{PackageInfo, SymbolNamer};
pub use scope::{ClassKind, ScopeFrame, ScopeStack};
