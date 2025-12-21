use std::{
    iter::{Repeat, repeat},
    marker::PhantomData,
};

use crate::{
    base::{Matchable, Matcher},
    utils::{Capture, Predicate},
};

#[derive(Debug, Clone)]
pub struct Digit<const N: u32 = 10>;

impl<const N: u32> Matchable<'_, usize> for Digit<N> {
    type Iter = Repeat<Predicate<char>>;
    fn m(&self) -> Matcher<Self::Iter, ()> {
        Matcher {
            iter: repeat(Predicate(|c: &char| c.to_digit(N).is_some())).peekable(),
            results: (),
            _marker: PhantomData,
        }
    }

    fn at_least(&self) -> usize {
        1
    }
}

impl<const N: u32> Capture<char, usize> for Digit<N> {
    fn capture(&self, consumed: Vec<char>) -> usize {
        let mut value: usize = 0;
        for c in consumed {
            if let Some(d) = c.to_digit(N) {
                value = value * (N as usize) + (d as usize);
            }
        }
        value
    }
}

#[derive(Debug, Clone)]
pub struct Words;

impl Matchable<'_, String> for Words {
    type Iter = Repeat<Predicate<char>>;
    fn m(&self) -> Matcher<Self::Iter, ()> {
        Matcher {
            iter: repeat(Predicate(|c: &char| c.is_alphanumeric())).peekable(),
            results: (),
            _marker: PhantomData,
        }
    }

    fn at_least(&self) -> usize {
        1
    }
}

impl Capture<char, String> for Words {
    fn capture(&self, consumed: Vec<char>) -> String {
        consumed.into_iter().collect()
    }
}

pub const NUM: Digit<10> = Digit::<10>;
pub const HEX: Digit<16> = Digit::<16>;
pub const OCT: Digit<8> = Digit::<8>;
pub const BIN: Digit<2> = Digit::<2>;
pub const WORDS: Words = Words;
