pub mod base;
pub mod dest;
pub mod exts;

// Re-exports to make core pattern types available at crate root for macro expansions
pub use base::{Checkpoint, Or, Pattern, Sep, Sep1, To};

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

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use match_string_macros::matches;

    use crate::{
        base::{Checkpoint, Sep, Seq},
        dest::Dest,
        exts::{ALPHABETIC, NUM},
    };

    use super::*;
    #[test]
    fn test_str_pattern() {
        let pattern = (Or("hello", "hi"), ", world!");
        let mut reference = Checkpoint::new("hi, world!".chars().peekable());
        let result = pattern.consume(&mut reference);
        println!("Result: {}", result);
        println!("Remaining: {}", reference.collect::<String>());
    }

    #[test]
    fn test_to_pattern() {
        let dest: Dest<String> = Dest::new();
        let pattern = (Or("hello", "hi"), To(", world!", &dest));
        let reference = "hi, world!";
        let result = pattern.matches(&reference);
        // same as let result = matches!(reference, "hello" / "hi", dest @ ", world!")
        println!("Result: {}", result);
        println!("Destination: {}", dest.borrow_mut());
    }

    #[test]
    fn test_seq_pattern() {
        let pattern = Seq(["hi", "ho", "ha"]);
        let mut reference = Checkpoint::new("hihoha".chars().peekable());
        let result = pattern.consume(&mut reference);
        println!("Result: {}", result);
        println!("Remaining: {}", reference.collect::<String>());
    }

    #[test]
    fn test_many_pattern() {
        let dest: Dest<Vec<String>> = Dest::new();
        let pattern = ..("wait", To("ing", &dest));
        let mut reference = Checkpoint::new("waitingwaitingdone".chars().peekable());
        let result = pattern.consume(&mut reference);
        println!("Result: {}", result);
        println!("Remaining: {}", reference.collect::<String>());
        println!("Destination: {:?}", dest.borrow_mut());
    }

    #[test]
    fn test_num_pattern() {
        let dest: Dest<Vec<usize>> = Dest::new();
        let dest2: Dest<Vec<(String, usize)>> = Dest::new();
        let pattern = To(..("a", NUM), &dest2);
        let mut reference = Checkpoint::new("a1a2a3done".chars().peekable());
        let result = pattern.consume(&mut reference);
        println!("Result: {}", result);
        println!("Remaining: {}", reference.collect::<String>());
        println!("Destination: {:?}", dest.borrow_mut());
        println!("Destination2: {:?}", dest2.borrow_mut());
    }

    #[test]
    fn test_macro_pattern() {
        let dest: Dest<Vec<usize>> = Dest::new();
        let dest2: Dest<Vec<String>> = Dest::new();
        let reference = "12,34,56.33";
        let result = matches!(reference => (dest@NUM)[dest2 @ ("," / ".")]+);
        println!("Result: {}", result);
        println!("Destination: {:?}", dest.borrow_mut());
        println!("Destination2: {:?}", dest2.borrow_mut());
    }

    #[test]
    fn test_token() {
        let dest: Dest<Vec<String>> = Dest::new();
        let result = matches!("func(arg, arg)" => "func(", (dest@ALPHABETIC)[","]+);
        println!("Result: {}", result);
        println!("Destination: {:?}", dest.borrow_mut());
    }
}
