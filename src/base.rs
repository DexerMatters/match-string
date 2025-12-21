use std::{
    fmt::{self, Debug},
    iter::Peekable,
    marker::PhantomData,
    ops, slice,
    str::Chars,
};

use crate::utils::{Capture, Result, ResultHList, Satisfies};

impl Capture<char, char> for char {
    fn capture(&self, consumed: Vec<char>) -> char {
        consumed.into_iter().next().unwrap_or(*self)
    }
}

pub trait Matchable<'a, Value = Self>: fmt::Debug
where
    Self: Sized,
{
    type Iter: Iterator + Clone + IntoIterator;

    fn m(&'a self) -> Matcher<Self::Iter, ()>
    where
        Peekable<Self::Iter>: Clone;

    fn is_recoverable(&self) -> bool {
        false
    }

    fn at_least(&self) -> usize {
        usize::MAX
    }

    fn at_most(&self) -> usize {
        usize::MAX
    }

    fn display_matched(&self) -> String {
        format!("{:?}", self)
    }
}

impl<'a, 'b> Matchable<'a, String> for &'b str {
    type Iter = Chars<'b>;

    fn m(&'a self) -> Matcher<Chars<'b>, ()> {
        Matcher {
            iter: self.chars().peekable(),
            results: (),
            _marker: PhantomData,
        }
    }
}

impl<'a, 'b> Capture<char, String> for &'b str {
    fn capture(&self, consumed: Vec<char>) -> String {
        consumed.into_iter().collect()
    }
}

impl Matchable<'_> for char {
    type Iter = std::iter::Once<char>;

    fn m(&self) -> Matcher<std::iter::Once<char>, ()> {
        Matcher {
            iter: std::iter::once(*self).peekable(),
            results: (),
            _marker: PhantomData,
        }
    }
}

impl<'a> Matchable<'a> for String {
    type Iter = Chars<'a>;

    fn m(&'a self) -> Matcher<Chars<'a>, ()> {
        Matcher {
            iter: self.chars().peekable(),
            results: (),
            _marker: PhantomData,
        }
    }
}

impl<'a, 'b, T: Eq + fmt::Debug + 'a> Matchable<'a> for &'b [T] {
    type Iter = slice::Iter<'a, T>;

    fn m(&'a self) -> Matcher<slice::Iter<'a, T>, ()> {
        Matcher {
            iter: self.iter().peekable(),
            results: (),
            _marker: PhantomData,
        }
    }
}

impl<'a, T: Eq + fmt::Debug + 'a> Matchable<'a> for Vec<T> {
    type Iter = slice::Iter<'a, T>;

    fn m(&'a self) -> Matcher<slice::Iter<'a, T>, ()> {
        Matcher {
            iter: self.iter().peekable(),
            results: (),
            _marker: PhantomData,
        }
    }
}

impl<'a, 'b, T: Eq + fmt::Debug + 'a> Matchable<'a> for &'b Vec<T> {
    type Iter = slice::Iter<'a, T>;

    fn m(&'a self) -> Matcher<slice::Iter<'a, T>, ()> {
        Matcher {
            iter: self.iter().peekable(),
            results: (),
            _marker: PhantomData,
        }
    }
}

pub struct Matcher<I, O>
where
    O: ResultHList,
    I: Iterator + Clone,
    Peekable<I>: Clone,
{
    pub(crate) iter: Peekable<I>,
    pub(crate) results: O,
    pub(crate) _marker: PhantomData<O>,
}

impl<I, O> Matcher<I, O>
where
    O: ResultHList,
    I: Iterator + Clone,
    Peekable<I>: Clone,
{
    pub fn results(&self) -> &O {
        &self.results
    }
    pub fn into_results(self) -> O {
        self.results
    }

    pub fn then<M, Value>(self, matchable: M) -> Matcher<I, O::AppendOpt<Value>>
    where
        M: for<'a> Matchable<'a, Value>,
        for<'a> <<M as Matchable<'a, Value>>::Iter as Iterator>::Item:
            Satisfies<<I as Iterator>::Item>,
        for<'a> Peekable<<M as Matchable<'a, Value>>::Iter>: Clone,
        I::Item: Clone,
        M: Capture<<I as Iterator>::Item, Value>,
        Value: fmt::Debug,
    {
        let is_recoverable = matchable.is_recoverable();
        let at_least = matchable.at_least();
        let at_most = matchable.at_most();

        let original_iter = if is_recoverable {
            Some(self.iter.clone())
        } else {
            None
        };

        let mut self_iter = self.iter;
        let mut consumed = 0;
        let mut consumed_items: Vec<I::Item> = Vec::new();

        let matched = if at_least == usize::MAX {
            // When at_least then MAX, the whole matchable must be satisfied
            let mut matcher_iter = matchable.m().iter;
            loop {
                match matcher_iter.next() {
                    None => break true,
                    Some(expected) => match self_iter.peek() {
                        Some(actual_ref) if expected.satisfies(actual_ref) => {
                            // consume the item
                            if let Some(it) = self_iter.next() {
                                consumed_items.push(it.clone());
                            }
                            consumed += 1;
                            continue;
                        }
                        _ => break false,
                    },
                }
            }
        } else {
            // Consume items between at_least and at_most
            let mut matcher_iter = matchable.m().iter;
            loop {
                if consumed >= at_most {
                    break true;
                }

                match matcher_iter.next() {
                    None => break consumed >= at_least,
                    Some(expected) => match self_iter.peek() {
                        Some(actual_ref) if expected.satisfies(actual_ref) => {
                            if let Some(it) = self_iter.next() {
                                consumed_items.push(it.clone());
                            }
                            consumed += 1;
                            continue;
                        }
                        _ => break consumed >= at_least,
                    },
                }
            }
        };

        let final_iter = if !matched && is_recoverable {
            original_iter.unwrap()
        } else {
            self_iter
        };

        let result_elem = if matched {
            // convert consumed_items into Value via Capture
            let v = matchable.capture(consumed_items);
            Result::Matched(v)
        } else if is_recoverable {
            Result::Skipped
        } else {
            Result::Error
        };

        Matcher {
            iter: final_iter,
            results: self.results.append_opt(result_elem),
            _marker: PhantomData,
        }
    }

    pub fn remaining(&self) -> Peekable<I> {
        self.iter.clone()
    }

    pub fn collect_remaining<V>(self) -> V
    where
        V: FromIterator<I::Item>,
    {
        self.iter.collect()
    }

    pub fn succeeded(&self) -> bool {
        self.results.all()
    }

    pub fn end(mut self) -> Matcher<I, O::AppendOpt<()>> {
        let is_end = self.iter.peek().is_none();
        Matcher {
            iter: self.iter.clone(),
            results: self.results.append_opt(if is_end {
                Result::Matched(())
            } else {
                Result::Error
            }),
            _marker: PhantomData,
        }
    }

    pub fn count(self) -> usize {
        self.iter.count()
    }
}

impl<I, O> fmt::Debug for Matcher<I, O>
where
    O: ResultHList + Debug,
    I: Iterator + Clone,
    I::Item: Eq,
    Peekable<I>: Clone,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Matcher")
            .field("results", &self.results)
            .finish()
    }
}

impl<I, O> fmt::Display for Matcher<I, O>
where
    O: ResultHList + Debug,
    I: Iterator + Clone,
    I::Item: Eq,
    Peekable<I>: Clone,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}]", self.results.format_results())
    }
}

pub struct M;

impl<'a, 'b> ops::Div<&'a str> for M {
    type Output = Matcher<Chars<'a>, ()>;
    fn div(self, rhs: &'a str) -> Self::Output {
        rhs.m()
    }
}

#[macro_export]
macro_rules! matches {
    ($input:expr, $($matcher:tt)+) => {
        {
            ($input.m()$(.then($matcher))+).succeeded()
        }
    };
}

#[cfg(test)]
mod tests {
    use crate::ext::{Digit, NUM, WORDS};

    use super::*;
    #[test]
    fn test_matchable_iterator() {
        let x = "https://example.com";
        let test = matches!(x, "https://" WORDS '.' WORDS);
        println!("{}", test);
    }
}
