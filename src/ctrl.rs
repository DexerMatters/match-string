use std::{fmt, iter::Peekable, marker::PhantomData, ops::RangeTo};

use crate::{
    base::{Matchable, Matcher},
    utils::Capture,
};

impl<'a, T, Value> Matchable<'a, Value> for (T,)
where
    T: fmt::Debug + Matchable<'a, Value>,
{
    type Iter = <T as Matchable<'a, Value>>::Iter;

    fn m(&'a self) -> Matcher<Self::Iter, ()>
    where
        Peekable<Self::Iter>: Clone,
    {
        Matcher {
            iter: self.0.m().iter,
            results: (),
            _marker: PhantomData,
        }
    }
    fn is_recoverable(&self) -> bool {
        true
    }
}

impl<Actual, Value, T> Capture<Actual, Value> for (T,)
where
    T: Capture<Actual, Value>,
{
    fn capture(&self, consumed: Vec<Actual>) -> Value {
        self.0.capture(consumed)
    }
}

impl<'a, T, Value> Matchable<'a, Vec<Value>> for RangeTo<T>
where
    T: fmt::Debug + Matchable<'a, Value>,
    <T as Matchable<'a, Value>>::Iter: Clone,
    for<'b> <<T as Matchable<'b, Value>>::Iter as Iterator>::Item: Clone,
{
    type Iter = std::iter::Cycle<Peekable<<T as Matchable<'a, Value>>::Iter>>;

    fn m(&'a self) -> Matcher<Self::Iter, ()>
    where
        Peekable<Self::Iter>: Clone,
    {
        let base = self.end.m().iter;
        Matcher {
            iter: base.cycle().peekable(),
            results: (),
            _marker: PhantomData,
        }
    }

    fn at_least(&self) -> usize {
        0
    }
}

impl<Actual, Value, T> Capture<Actual, Vec<Value>> for RangeTo<T>
where
    T: Capture<Actual, Value> + for<'a> Matchable<'a, Value>,
    for<'a> Peekable<<T as Matchable<'a, Value>>::Iter>: Clone,
    Actual: Clone,
{
    fn capture(&self, consumed: Vec<Actual>) -> Vec<Value> {
        // determine how many actual items make up one occurrence of the inner matcher
        let per_occurrence = (&self.end).m().iter.count();

        if per_occurrence == 0 {
            // fallback: treat each actual as a separate occurrence
            return consumed
                .into_iter()
                .map(|a| self.end.capture(vec![a]))
                .collect();
        }

        consumed
            .chunks(per_occurrence)
            .map(|chunk| self.end.capture(chunk.to_vec()))
            .collect()
    }
}
