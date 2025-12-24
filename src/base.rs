use core::slice;
use std::{
    cell::RefCell,
    ops::{RangeTo, RangeToInclusive},
    str::Chars,
};

use crate::dest;

use std::collections::VecDeque;

/// An iterator wrapper that supports checkpointing (snapshots and rollbacks).
pub struct Checkpoint<I>
where
    I: Iterator,
{
    inner: I,
    front: VecDeque<I::Item>,
    trail: Vec<I::Item>,
    in_trial: bool,
}

impl<I> Checkpoint<I>
where
    I: Iterator,
    I::Item: Clone,
{
    pub fn new(inner: I) -> Self {
        Checkpoint {
            inner,
            front: VecDeque::new(),
            trail: Vec::new(),
            in_trial: false,
        }
    }

    pub fn begin(&mut self) {
        self.trail.clear();
        self.in_trial = true;
    }

    pub fn commit(&mut self) {
        self.trail.clear();
        self.in_trial = false;
    }

    pub fn rollback(&mut self) {
        // Move trail items to the front in original order
        while let Some(it) = self.trail.pop() {
            self.front.push_front(it);
        }
        self.in_trial = false;
    }
}

impl<I> Iterator for Checkpoint<I>
where
    I: Iterator,
    I::Item: Clone,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(it) = self.front.pop_front() {
            return Some(it);
        }
        match self.inner.next() {
            Some(it) => {
                if self.in_trial {
                    // store a clone for potential rollback and return the original
                    self.trail.push(it.clone());
                    Some(it)
                } else {
                    Some(it)
                }
            }
            None => None,
        }
    }
}

impl<I> PeekableExt for Checkpoint<I>
where
    I: Iterator + PeekableExt,
    I::Item: Clone,
{
    fn peek(&mut self) -> Option<&Self::Item> {
        if let Some(front) = self.front.front() {
            return Some(front);
        }
        self.inner.peek()
    }
}

impl<I> Clone for Checkpoint<I>
where
    I: Iterator + Clone,
    I::Item: Clone,
{
    fn clone(&self) -> Self {
        Checkpoint {
            inner: self.inner.clone(),
            front: self.front.clone(),
            trail: self.trail.clone(),
            in_trial: self.in_trial,
        }
    }
}

/// A trait for types that can be checked for satisfaction against another type.
pub trait Satisfies<T> {
    fn satisfies(&self, item: &T) -> bool;
}

impl<T> Satisfies<T> for T
where
    T: PartialEq,
{
    fn satisfies(&self, item: &T) -> bool {
        self == item
    }
}

/// A trait for types that can receive matched items.
pub trait Destination<Item> {
    fn pickup(&mut self, _item: Item) {}
}

impl<T> Destination<&T> for Vec<T>
where
    T: Clone,
{
    fn pickup(&mut self, item: &T) {
        self.push(item.clone());
    }
}

impl Destination<char> for String {
    fn pickup(&mut self, item: char) {
        self.push(item);
    }
}

impl<T> Destination<char> for Vec<T> where T: Destination<char> {}

/// A trait for iterable reference types.
pub trait Iterable<'a> {
    type Iter: Iterator;
    /// Get an iterator over the reference.
    fn get_iter(&'a self) -> Self::Iter;
}

/// A trait for iterators that support peeking at the next item without consuming it.
pub trait PeekableExt: Iterator {
    /// Peek at the next item without consuming it.
    fn peek(&mut self) -> Option<&Self::Item>;
}

impl<I> PeekableExt for std::iter::Peekable<I>
where
    I: Iterator,
{
    fn peek(&mut self) -> Option<&Self::Item> {
        std::iter::Peekable::peek(self)
    }
}

/// A trait for pattern types that can match against a reference iterator.
pub trait Pattern<'a, Reference>
where
    Reference: Iterator,
{
    type Iter: Iterator;
    type Dest;
    /// Get an iterator over the pattern's items.
    fn get_iter(&'a self) -> Self::Iter;
    /// Get a mutable reference to the pattern's internal destination, if any.
    fn get_dest_mut(&self) -> Option<std::cell::RefMut<'_, Self::Dest>> {
        None
    }
    /// Match the pattern against the reference iterator.
    fn matches<'s, R>(&'a self, reference: &'s R) -> bool
    where
        R: Iterable<'s, Iter = Reference> + 's,
        Self::Dest: Destination<Reference::Item>,
        Reference: PeekableExt,
        Reference::Item: Satisfies<<Self::Iter as Iterator>::Item>,
    {
        let mut iter = reference.get_iter();
        self.consume(&mut iter) && iter.peek().is_none()
    }
    /// Consume items from the reference iterator, optionally storing matched items in a destination.
    fn consume_with_dest(
        &'a self,
        reference_iter: &mut Reference,
        dest: Option<&RefCell<Self::Dest>>,
    ) -> bool
    where
        Reference: PeekableExt,
        Self::Dest: Destination<Reference::Item>,
        Reference::Item: Satisfies<<Self::Iter as Iterator>::Item>,
    {
        // Use either the provided destination or the pattern's internal one,
        // then run a single consumption loop (avoids duplicating logic).
        let mut maybe_dest = dest
            .map(|dref| dref.borrow_mut())
            .or_else(|| self.get_dest_mut());
        self.get_iter().all(|pat_item| match reference_iter.peek() {
            Some(item) if item.satisfies(&pat_item) => {
                if let Some(consumed) = reference_iter.next() {
                    if let Some(d) = maybe_dest.as_mut() {
                        d.pickup(consumed)
                    }
                    true
                } else {
                    false
                }
            }
            _ => false,
        })
    }

    /// Consume items from the reference iterator.
    fn consume(&'a self, reference_iter: &mut Reference) -> bool
    where
        Reference: PeekableExt,
        Self::Dest: Destination<Reference::Item>,
        Reference::Item: Satisfies<<Self::Iter as Iterator>::Item>,
    {
        self.consume_with_dest(reference_iter, None)
    }
}

impl<'a> Iterable<'a> for &'a str {
    type Iter = Checkpoint<std::iter::Peekable<Chars<'a>>>;
    fn get_iter(&'a self) -> Self::Iter {
        Checkpoint::new(self.chars().peekable())
    }
}

impl<'a, Reference> Pattern<'a, Reference> for &'a str
where
    Reference: Iterator<Item = char> + PeekableExt,
{
    type Iter = Checkpoint<std::iter::Peekable<Chars<'a>>>;
    type Dest = String;
    fn get_iter(&'a self) -> Self::Iter {
        Checkpoint::new(self.chars().peekable())
    }
}

impl<'a> Iterable<'a> for String {
    type Iter = Checkpoint<std::iter::Peekable<Chars<'a>>>;
    fn get_iter(&'a self) -> Self::Iter {
        Checkpoint::new(self.chars().peekable())
    }
}

impl<'a, Reference> Pattern<'a, Reference> for String
where
    Reference: Iterator<Item = char> + PeekableExt,
{
    type Iter = Checkpoint<std::iter::Peekable<Chars<'a>>>;
    type Dest = String;
    fn get_iter(&'a self) -> Self::Iter {
        Checkpoint::new(self.chars().peekable())
    }
}

impl<'a, T> Iterable<'a> for &'a [T]
where
    T: 'a,
{
    type Iter = std::iter::Peekable<slice::Iter<'a, T>>;
    fn get_iter(&'a self) -> Self::Iter {
        self.iter().peekable()
    }
}

impl<'a, T, Reference> Pattern<'a, Reference> for &'a [T]
where
    T: 'a,
    Reference: Iterator<Item = &'a T> + PeekableExt,
    T: Clone,
    Reference::Item: Satisfies<&'a T>,
{
    type Iter = std::iter::Peekable<slice::Iter<'a, T>>;
    type Dest = Vec<T>;
    fn get_iter(&'a self) -> Self::Iter {
        self.iter().peekable()
    }
}

impl<'a, T> Iterable<'a> for Vec<T>
where
    T: 'a,
{
    type Iter = std::iter::Peekable<slice::Iter<'a, T>>;
    fn get_iter(&'a self) -> Self::Iter {
        self.iter().peekable()
    }
}

impl<'a, T, Reference> Pattern<'a, Reference> for Vec<T>
where
    T: 'a,
    Reference: Iterator<Item = &'a T> + PeekableExt,
    T: Clone,
    Reference::Item: Satisfies<&'a T>,
{
    type Iter = std::iter::Peekable<slice::Iter<'a, T>>;
    type Dest = Vec<T>;
    fn get_iter(&'a self) -> Self::Iter {
        self.iter().peekable()
    }
}

/// A pattern that matches either of two sub-patterns.
pub struct Or<A, B>(pub A, pub B);

impl<'a, Reference, A, B, D> Pattern<'a, Reference> for Or<A, B>
where
    Reference: Iterator + Clone + PeekableExt,
    A: Pattern<'a, Reference, Dest = D>,
    B: Pattern<'a, Reference, Dest = D>,
    D: Destination<Reference::Item> + Clone,
    Reference::Item: Satisfies<<<A as Pattern<'a, Reference>>::Iter as Iterator>::Item>,
    Reference::Item: Satisfies<<<B as Pattern<'a, Reference>>::Iter as Iterator>::Item>,
{
    type Iter = std::iter::Peekable<core::iter::Empty<Reference::Item>>;
    type Dest = D;

    fn get_iter(&'a self) -> Self::Iter {
        core::iter::empty().peekable()
    }

    fn consume(&'a self, reference: &mut Reference) -> bool
    where
        Reference::Item: Satisfies<<Self::Iter as Iterator>::Item>,
    {
        // Delegate to the more general `consume_with_dest`, avoiding
        // duplicate snapshot/restore logic.
        self.consume_with_dest(reference, None)
    }

    fn consume_with_dest(
        &'a self,
        reference: &mut Reference,
        dest: Option<&RefCell<Self::Dest>>,
    ) -> bool
    where
        Reference::Item: Satisfies<<Self::Iter as Iterator>::Item>,
    {
        let orig = reference.clone();

        // Snapshot any provided dest value so we can restore on failure
        let provided_backup = dest.as_ref().map(|d| d.borrow().clone());

        // Try A: take a brief borrow to snapshot internal dest if available
        let a_internal_backup = match self.0.get_dest_mut() {
            Some(d) => {
                let b = d.clone();
                drop(d);
                Some(b)
            }
            None => None,
        };

        if A::consume_with_dest(&self.0, reference, dest) {
            return true;
        }

        *reference = orig.clone();

        if let Some(b) = a_internal_backup {
            if let Some(mut d) = self.0.get_dest_mut() {
                *d = b.clone();
            }
        }

        if let Some(b) = provided_backup.clone() {
            if let Some(dref) = dest {
                *dref.borrow_mut() = b;
            }
        }

        // Try B: snapshot (may be same underlying dest)
        let b_internal_backup = match self.1.get_dest_mut() {
            Some(d) => {
                let b = d.clone();
                drop(d);
                Some(b)
            }
            None => None,
        };

        if B::consume_with_dest(&self.1, reference, dest) {
            return true;
        }

        *reference = orig;

        if let Some(b) = b_internal_backup {
            if let Some(mut d) = self.1.get_dest_mut() {
                *d = b;
            }
        }

        if let Some(b) = provided_backup {
            if let Some(dref) = dest {
                *dref.borrow_mut() = b;
            }
        }

        false
    }
}

impl<'a, Reference, A, B, DA, DB> Pattern<'a, Reference> for (A, B)
where
    Reference: Iterator + PeekableExt,
    A: Pattern<'a, Reference, Dest = DA>,
    B: Pattern<'a, Reference, Dest = DB>,
    DA: Destination<Reference::Item> + Clone,
    DB: Destination<Reference::Item> + Clone,
    Reference::Item: Satisfies<<<A as Pattern<'a, Reference>>::Iter as Iterator>::Item>,
    Reference::Item: Satisfies<<<B as Pattern<'a, Reference>>::Iter as Iterator>::Item>,
    Reference::Item: Clone,
{
    type Iter = std::iter::Peekable<core::iter::Empty<Reference::Item>>;
    type Dest = (DA, DB);

    fn get_iter(&'a self) -> Self::Iter {
        core::iter::empty().peekable()
    }

    fn consume(&'a self, reference: &mut Reference) -> bool
    where
        Reference::Item: Satisfies<<Self::Iter as Iterator>::Item>,
    {
        self.0.consume(reference) && self.1.consume(reference)
    }

    fn consume_with_dest(
        &'a self,
        reference_iter: &mut Reference,
        dest: Option<&RefCell<Self::Dest>>,
    ) -> bool
    where
        Reference::Item: Satisfies<<Self::Iter as Iterator>::Item>,
    {
        if let Some(dref) = dest {
            // snapshot current tuple dest
            let snapshot = dref.borrow().clone();
            // create inner temp dests
            let a_temp = RefCell::new(snapshot.0);
            let b_temp = RefCell::new(snapshot.1);

            // try to consume both parts routing to temp dests
            if A::consume_with_dest(&self.0, reference_iter, Some(&a_temp))
                && B::consume_with_dest(&self.1, reference_iter, Some(&b_temp))
            {
                // commit back into original dest
                let mut d = dref.borrow_mut();
                d.0 = a_temp.into_inner();
                d.1 = b_temp.into_inner();
                return true;
            }
            return false;
        }

        self.consume(reference_iter)
    }
}

/// A trait for types that can collect captured items into a destination.
pub trait Collector<Inner, Item> {
    fn commit(out: &RefCell<Self>, captured: Inner);
}

impl<Inner, Item> Collector<Inner, Item> for Inner
where
    Inner: Destination<Item> + Clone,
{
    fn commit(out: &RefCell<Self>, captured: Inner) {
        *out.borrow_mut() = captured;
    }
}

impl<Inner, Item> Collector<Inner, Item> for Vec<Inner>
where
    Inner: Destination<Item> + Default + Clone,
{
    fn commit(out: &RefCell<Self>, captured: Inner) {
        out.borrow_mut().push(captured);
    }
}

/// A pattern that captures matched items into a destination.
pub struct To<'a, A, D>(pub A, pub &'a dest::Dest<D>);

impl<'a, Reference, A, OutD, InD> Pattern<'a, Reference> for To<'a, A, OutD>
where
    Reference: Iterator + PeekableExt,
    A: Pattern<'a, Reference, Dest = InD>,
    InD: Destination<Reference::Item> + Default + Clone,
    OutD: Collector<InD, Reference::Item> + Destination<Reference::Item> + Clone,
{
    type Iter = <A as Pattern<'a, Reference>>::Iter;
    type Dest = OutD;

    fn get_iter(&'a self) -> Self::Iter {
        self.0.get_iter()
    }

    fn get_dest_mut(&self) -> Option<std::cell::RefMut<'_, Self::Dest>> {
        Some(self.1.borrow_mut())
    }

    fn consume(&'a self, reference: &mut Reference) -> bool
    where
        Reference::Item: Satisfies<<Self::Iter as Iterator>::Item>,
    {
        self.consume_with_dest(reference, None)
    }

    fn consume_with_dest(
        &'a self,
        reference: &mut Reference,
        dest: Option<&RefCell<Self::Dest>>,
    ) -> bool
    where
        Reference: PeekableExt,
        Reference::Item: Satisfies<<Self::Iter as Iterator>::Item>,
    {
        let outer = self.1.as_refcell();
        let provided_target = match dest {
            Some(d) => d,
            None => outer,
        };

        let inner_temp = RefCell::new(InD::default());
        if A::consume_with_dest(&self.0, reference, Some(&inner_temp)) {
            let captured = inner_temp.into_inner();
            <OutD as Collector<InD, Reference::Item>>::commit(provided_target, captured.clone());
            if !std::ptr::eq(
                provided_target as *const RefCell<OutD>,
                outer as *const RefCell<OutD>,
            ) {
                <OutD as Collector<InD, Reference::Item>>::commit(outer, captured);
            }

            true
        } else {
            false
        }
    }
}

/// A pattern that matches a sequence of sub-patterns.
pub struct Seq<A, const N: usize>(pub [A; N]);

impl<'a, Reference, A, D, const N: usize> Pattern<'a, Reference> for Seq<A, N>
where
    Reference: Iterator + Clone + PeekableExt,
    A: Pattern<'a, Reference, Dest = D>,
    D: Destination<Reference::Item> + Default,
    Reference::Item: Satisfies<<<A as Pattern<'a, Reference>>::Iter as Iterator>::Item>,
{
    type Iter = core::iter::Peekable<core::iter::Empty<Reference::Item>>;
    type Dest = Vec<D>;

    fn get_iter(&'a self) -> Self::Iter {
        core::iter::empty().peekable()
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

        let mut temp: Vec<D> = Vec::new();

        for child in &self.0 {
            let inner = RefCell::new(D::default());
            if !A::consume_with_dest(child, &mut trial, Some(&inner)) {
                return false;
            }
            temp.push(inner.into_inner());
        }

        *reference = trial;

        if let Some(dref) = dest {
            let mut d = dref.borrow_mut();
            d.extend(temp);
        }

        true
    }
}

impl<Item, A, B> Destination<Item> for (A, B)
where
    A: Destination<Item>,
    B: Destination<Item>,
    Item: Clone,
{
    fn pickup(&mut self, item: Item) {
        let a_item = item.clone();
        self.0.pickup(a_item);
        self.1.pickup(item);
    }
}

impl<'a, Reference, A, D> Pattern<'a, Reference> for RangeTo<A>
where
    Reference: Iterator + Clone + PeekableExt,
    A: Pattern<'a, Reference, Dest = D>,
    D: Destination<Reference::Item> + Default + Clone,
    Reference::Item: Satisfies<<<A as Pattern<'a, Reference>>::Iter as Iterator>::Item> + Clone,
{
    type Iter = core::iter::Empty<Reference::Item>;
    type Dest = Vec<D>;

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
        match dest {
            Some(dref) => {
                loop {
                    let mut trial = reference.clone();
                    let inner_dest = RefCell::new(D::default());
                    if !A::consume_with_dest(&self.end, &mut trial, Some(&inner_dest)) {
                        break;
                    }
                    // compute how many items trial consumed by comparing remainders
                    let rem_orig: Vec<Reference::Item> = reference.clone().collect();
                    let rem_trial: Vec<Reference::Item> = trial.clone().collect();
                    let consumed = rem_orig.len().saturating_sub(rem_trial.len());
                    if consumed == 0 {
                        break;
                    }
                    for _ in 0..consumed {
                        reference.next();
                    }
                    dref.borrow_mut().push(inner_dest.into_inner());
                }
                true
            }
            None => {
                loop {
                    let mut trial = reference.clone();
                    if !A::consume(&self.end, &mut trial) {
                        break;
                    }
                    let rem_orig: Vec<Reference::Item> = reference.clone().collect();
                    let rem_trial: Vec<Reference::Item> = trial.clone().collect();
                    let consumed = rem_orig.len().saturating_sub(rem_trial.len());
                    if consumed == 0 {
                        break;
                    }
                    for _ in 0..consumed {
                        reference.next();
                    }
                }
                true
            }
        }
    }
}

impl<'a, Reference, A, D> Pattern<'a, Reference> for RangeToInclusive<A>
where
    Reference: Iterator + Clone + PeekableExt,
    A: Pattern<'a, Reference, Dest = D>,
    D: Destination<Reference::Item> + Default + Clone,
    Reference::Item: Satisfies<<<A as Pattern<'a, Reference>>::Iter as Iterator>::Item> + Clone,
{
    type Iter = core::iter::Empty<Reference::Item>;
    type Dest = Vec<D>;

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
        // require at least one successful match; empty-child guard removed

        match dest {
            Some(dref) => {
                let mut any = false;
                loop {
                    let mut trial = reference.clone();
                    let inner_dest = RefCell::new(D::default());
                    if !A::consume_with_dest(&self.end, &mut trial, Some(&inner_dest)) {
                        break;
                    }
                    let rem_orig: Vec<Reference::Item> = reference.clone().collect();
                    let rem_trial: Vec<Reference::Item> = trial.clone().collect();
                    let consumed = rem_orig.len().saturating_sub(rem_trial.len());
                    if consumed == 0 {
                        break;
                    }
                    for _ in 0..consumed {
                        reference.next();
                    }
                    dref.borrow_mut().push(inner_dest.into_inner());
                    any = true;
                }
                any
            }
            None => {
                let mut any = false;
                loop {
                    let mut trial = reference.clone();
                    if !A::consume(&self.end, &mut trial) {
                        break;
                    }
                    let rem_orig: Vec<Reference::Item> = reference.clone().collect();
                    let rem_trial: Vec<Reference::Item> = trial.clone().collect();
                    let consumed = rem_orig.len().saturating_sub(rem_trial.len());
                    if consumed == 0 {
                        break;
                    }
                    for _ in 0..consumed {
                        reference.next();
                    }
                    any = true;
                }
                any
            }
        }
    }
}

/// A pattern that matches a sequence of sub-patterns separated by a separator pattern.
pub struct Sep<Sep, P>(pub Sep, pub P);

/// A pattern that matches one or more occurrences of a sub-pattern separated by a separator pattern.
pub struct Sep1<Sep, P>(pub Sep, pub P);

impl<'a, Reference, SepT, PatT, SD, PD> Pattern<'a, Reference> for Sep<SepT, PatT>
where
    Reference: Iterator + Clone + PeekableExt,
    SepT: Pattern<'a, Reference, Dest = SD>,
    PatT: Pattern<'a, Reference, Dest = PD>,
    SD: Destination<Reference::Item> + Default + Clone,
    PD: Destination<Reference::Item> + Default + Clone,
    Reference::Item: Satisfies<<<SepT as Pattern<'a, Reference>>::Iter as Iterator>::Item>
        + Satisfies<<<PatT as Pattern<'a, Reference>>::Iter as Iterator>::Item>
        + Clone,
{
    type Iter = core::iter::Empty<Reference::Item>;
    type Dest = Vec<(SD, PD)>;

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
        // Collect matches into a temp vector, committing only on success
        let mut temp: Vec<(SD, PD)> = Vec::new();

        loop {
            // Try to parse a pattern occurrence (for Sep this may fail immediately)
            let mut trial = reference.clone();
            let pat_dest = RefCell::new(PD::default());
            if !PatT::consume_with_dest(&self.1, &mut trial, Some(&pat_dest)) {
                break;
            }

            // Try to parse a separator following the pattern; separator may be absent
            let sep_dest = RefCell::new(SD::default());
            let mut trial_after_sep = trial.clone();
            if SepT::consume_with_dest(&self.0, &mut trial_after_sep, Some(&sep_dest)) {
                // separator consumed; advance trial to after separator
                trial = trial_after_sep;
            }

            // compute how many items were consumed and advance the real iterator
            let rem_orig: Vec<Reference::Item> = reference.clone().collect();
            let rem_trial: Vec<Reference::Item> = trial.clone().collect();
            let consumed = rem_orig.len().saturating_sub(rem_trial.len());
            if consumed == 0 {
                break;
            }
            for _ in 0..consumed {
                reference.next();
            }

            temp.push((sep_dest.into_inner(), pat_dest.into_inner()));
        }

        if let Some(dref) = dest {
            let mut d = dref.borrow_mut();
            d.extend(temp);
        }

        true
    }
}

impl<'a, Reference, SepT, PatT, SD, PD> Pattern<'a, Reference> for Sep1<SepT, PatT>
where
    Reference: Iterator + Clone + PeekableExt,
    SepT: Pattern<'a, Reference, Dest = SD>,
    PatT: Pattern<'a, Reference, Dest = PD>,
    SD: Destination<Reference::Item> + Default + Clone,
    PD: Destination<Reference::Item> + Default + Clone,
    Reference::Item: Satisfies<<<SepT as Pattern<'a, Reference>>::Iter as Iterator>::Item>
        + Satisfies<<<PatT as Pattern<'a, Reference>>::Iter as Iterator>::Item>
        + Clone,
{
    type Iter = core::iter::Empty<Reference::Item>;
    type Dest = Vec<(SD, PD)>;

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
        // Require at least one occurrence
        let mut temp: Vec<(SD, PD)> = Vec::new();

        // First element must be a pattern
        let mut trial = reference.clone();
        let first_pat = RefCell::new(PD::default());
        if !PatT::consume_with_dest(&self.1, &mut trial, Some(&first_pat)) {
            return false;
        }

        // Try optional separator after first element
        let first_sep = RefCell::new(SD::default());
        let mut trial_after_sep = trial.clone();
        if SepT::consume_with_dest(&self.0, &mut trial_after_sep, Some(&first_sep)) {
            trial = trial_after_sep;
        }

        // advance real iterator and record
        let rem_orig: Vec<Reference::Item> = reference.clone().collect();
        let rem_trial: Vec<Reference::Item> = trial.clone().collect();
        let consumed = rem_orig.len().saturating_sub(rem_trial.len());
        for _ in 0..consumed {
            reference.next();
        }
        temp.push((first_sep.into_inner(), first_pat.into_inner()));

        // subsequent (sep, pat)*
        loop {
            let mut trial = reference.clone();

            // try separator then pattern
            let sep_temp = RefCell::new(SD::default());
            if !SepT::consume_with_dest(&self.0, &mut trial, Some(&sep_temp)) {
                break;
            }
            let pat_temp = RefCell::new(PD::default());
            if !PatT::consume_with_dest(&self.1, &mut trial, Some(&pat_temp)) {
                break;
            }

            let rem_orig: Vec<Reference::Item> = reference.clone().collect();
            let rem_trial: Vec<Reference::Item> = trial.clone().collect();
            let consumed = rem_orig.len().saturating_sub(rem_trial.len());
            if consumed == 0 {
                break;
            }
            for _ in 0..consumed {
                reference.next();
            }

            temp.push((sep_temp.into_inner(), pat_temp.into_inner()));
        }

        if let Some(dref) = dest {
            let mut d = dref.borrow_mut();
            d.extend(temp);
        }

        true
    }
}
