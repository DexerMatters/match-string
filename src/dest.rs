use std::cell::{RefCell, RefMut};

/// A destination that can store matched items.
/// This is a wrapper around `RefCell<T>` to allow interior mutability
/// while providing a convenient API for pattern matching destinations.
#[derive(Default)]
pub struct Dest<T> {
    inner: RefCell<T>,
}

impl<T> Dest<T> {
    pub fn new() -> Self
    where
        T: Default,
    {
        Default::default()
    }
    pub fn borrow_mut(&self) -> RefMut<'_, T> {
        self.inner.borrow_mut()
    }

    pub fn as_refcell(&self) -> &RefCell<T> {
        &self.inner
    }

    pub fn into_inner(self) -> T {
        self.inner.into_inner()
    }
}

impl<T> From<T> for Dest<T> {
    fn from(value: T) -> Self {
        Dest {
            inner: RefCell::new(value),
        }
    }
}

impl<T> Clone for Dest<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        self.inner.borrow().clone().into()
    }
}

impl<T> std::fmt::Debug for Dest<T>
where
    T: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Dest").field(&self.inner.borrow()).finish()
    }
}

impl<Item, T> crate::base::Destination<Item> for Dest<T>
where
    T: crate::base::Destination<Item>,
{
    fn pickup(&mut self, item: Item) {
        self.inner.borrow_mut().pickup(item)
    }
}
