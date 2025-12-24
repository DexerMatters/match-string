use crate::base::{Destination, Pattern, PeekableExt, Satisfies};
use std::cell::RefCell;

pub struct Token<Ref, Dest> {
    pub predicate: fn(&Ref) -> bool,
    pub parser: fn(Vec<Ref>) -> Dest,
    pub at_least: usize,
    pub skip_leading: Option<fn(&Ref) -> bool>,
}

impl<'a, Reference, RefT, D> Pattern<'a, Reference> for Token<RefT, D>
where
    Reference: Iterator<Item = RefT> + Clone + PeekableExt,
{
    type Iter = core::iter::Empty<Reference::Item>;
    type Dest = D;

    fn get_iter(&'a self) -> Self::Iter {
        core::iter::empty()
    }

    fn consume_with_dest(
        &'a self,
        reference: &mut Reference,
        dest: Option<&RefCell<Self::Dest>>,
    ) -> bool
    where
        Reference::Item: Satisfies<<Self::Iter as Iterator>::Item>,
    {
        let mut trial = reference.clone();
        if let Some(skip) = self.skip_leading {
            while let Some(p) = trial.peek() {
                if skip(p) {
                    let _ = trial.next();
                    continue;
                }
                break;
            }
        }
        let mut collected: Vec<RefT> = Vec::new();

        while let Some(peeked) = trial.peek() {
            if (self.predicate)(peeked) {
                if let Some(next_item) = trial.next() {
                    collected.push(next_item);
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        if collected.len() < self.at_least {
            return false;
        }
        let mut advance = collected.len();
        if let Some(skip) = self.skip_leading {
            let mut temp = reference.clone();
            let mut skipped = 0usize;
            while let Some(p) = temp.peek() {
                if skip(p) {
                    let _ = temp.next();
                    skipped += 1;
                    continue;
                }
                break;
            }
            advance += skipped;
        }
        for _ in 0..advance {
            reference.next();
        }

        // Parse the collected slice into the destination value
        let parsed = (self.parser)(collected);

        if let Some(dref) = dest {
            *dref.borrow_mut() = parsed;
        }

        true
    }
}

impl Destination<char> for usize {}

// Numeric token helpers (parametric by base `N`).
fn pred_num<const N: u32>(ch: &char) -> bool {
    ch.to_digit(N).is_some()
}

fn parse_num<const N: u32>(v: Vec<char>) -> usize {
    v.into_iter().fold(0usize, |acc, c| {
        acc.saturating_mul(N as usize)
            .saturating_add(c.to_digit(N).unwrap() as usize)
    })
}

const fn make_num<const N: u32>() -> Token<char, usize> {
    Token {
        predicate: pred_num::<N>,
        parser: parse_num::<N>,
        at_least: 1,
        skip_leading: None,
    }
}

pub const NUM: Token<char, usize> = make_num::<10>();
pub const HEX: Token<char, usize> = make_num::<16>();
pub const OCT: Token<char, usize> = make_num::<8>();
pub const BIN: Token<char, usize> = make_num::<2>();

pub const WS: Token<char, ()> = Token {
    predicate: |ch| ch.is_whitespace(),
    parser: |_| (),
    at_least: 1,
    skip_leading: None,
};

pub const ALPHABETIC: Token<char, String> = Token {
    predicate: |ch| ch.is_alphabetic(),
    parser: |v| v.into_iter().collect(),
    at_least: 1,
    skip_leading: None,
};

pub const ALPHANUMERIC: Token<char, String> = Token {
    predicate: |ch| ch.is_alphanumeric(),
    parser: |v| v.into_iter().collect(),
    at_least: 1,
    skip_leading: None,
};
