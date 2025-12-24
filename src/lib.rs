pub mod base;
pub mod dest;
pub mod exts;

// Re-exports to make core pattern types available at crate root for macro expansions
pub use base::{Checkpoint, Or, Pattern, Sep, Sep1, To};
pub use match_string_macros::matches;

/// Internal helper used by the proc-macro to call the `Pattern::matches` method
/// with the correct trait bounds so method resolution succeeds in macro expansions.
pub fn __matches<'a, 's, P, Reference, R>(pat: &'a P, reference: &'s R) -> bool
where
    P: crate::base::Pattern<'a, Reference>,
    R: crate::base::Iterable<'s, Iter = Reference> + 's,
    Reference: crate::base::PeekableExt,
    P::Dest: crate::base::Destination<Reference::Item>,
    Reference::Item: crate::base::Satisfies<<P::Iter as Iterator>::Item>,
{
    <P as crate::base::Pattern<'a, Reference>>::matches(pat, reference)
}
