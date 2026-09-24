pub(crate) mod handler;
pub mod ir;
pub(crate) mod parser;
pub(crate) mod render;
pub mod rir;

pub use ir::IrMathSource;
pub use rir::{RMathExpr, RMathResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MathMode {
    Display,
    Compute,
    Both, // this name is temporary.
}
