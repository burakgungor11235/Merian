/*!
 THIS IS A WIP
 normally this needs to get a concluded IR, but since we don't need one right now and are
 trying to get a vertical slice, I'm just going to make a plain old asp serializer and then add
 the need for an IR (functions / directives) and then do the IR and then come back. 
*/

use merian_core::error::DummyError;
use merian_frontend::ast::Asp;

#[derive(Default)]
/// Shared state available to every lowering backend.
pub struct LowerContext {
    pub diagnostics: DummyError, // for now.

    // Future:
    // pub source_map: SourceMap,
    // pub symbols: SymbolTable,
    // pub interner: StringInterner,
    // ...
}



/// Converts one compiler representation into another.
///
/// A lowerer may emit diagnostics while producing an output.
pub trait Lowerer {
    /// Backend-specific output.
    type Output;

    /// Backend version.
    const VERSION: &'static str;

    /// Human-readable backend name.
    const NAME: &'static str;

    /// Perform lowering.
    fn lower(
        &self,
        ast: &Asp,
        ctx: &mut LowerContext,
    ) -> Option<Self::Output>;
}
