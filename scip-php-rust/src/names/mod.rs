//! PHP name resolution: use statements, namespace context, FQN construction.

pub mod resolver;
pub mod traversal;

pub use resolver::{
    ClassResolution, ConstantResolution, FunctionResolution, NameContext, NameResolution,
    NameResolver, UseImport, UseKind,
};
pub use traversal::FileNameResolver;
