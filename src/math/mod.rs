pub mod debug;
pub mod expr;
pub(crate) mod handler;
pub mod ir;
pub(crate) mod parser;
pub mod rir;
pub mod scope;
pub mod solve;

pub use ir::IrMathSource;
pub use rir::{RMathExpr, RMathResult};
pub use scope::{ScopeArena, ScopeId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MathMode {
    Display,
    Compute,
    Both,
}
