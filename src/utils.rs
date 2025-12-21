use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Result<M> {
    Skipped,
    Error,
    Matched(M),
}

pub trait HList {
    type Head;
    type Tail: HList;
    fn head(self) -> Self::Head;
    fn tail(self) -> Self::Tail;
}

pub trait ResultHList: HList {
    type AppendOpt<Elem: fmt::Debug>: ResultHList;
    fn all(&self) -> bool;
    fn any(&self) -> bool;
    fn append_opt<Elem: fmt::Debug>(self, elem: Result<Elem>) -> Self::AppendOpt<Elem>;
    fn format_results(&self) -> String;
}

impl HList for () {
    type Head = ();
    type Tail = ();
    fn head(self) -> Self::Head {
        self
    }
    fn tail(self) -> Self::Tail {
        self
    }
}

impl ResultHList for () {
    type AppendOpt<Elem: fmt::Debug> = ((), Result<Elem>);
    fn all(&self) -> bool {
        true
    }
    fn any(&self) -> bool {
        false
    }
    fn append_opt<Elem: fmt::Debug>(self, elem: Result<Elem>) -> Self::AppendOpt<Elem> {
        ((), elem)
    }
    fn format_results(&self) -> String {
        String::new()
    }
}

impl<Head, Tail: HList> HList for (Tail, Head) {
    type Head = Head;
    type Tail = Tail;
    fn head(self) -> Self::Head {
        self.1
    }
    fn tail(self) -> Self::Tail {
        self.0
    }
}

impl<Head: fmt::Debug, Tail: ResultHList> ResultHList for (Tail, Result<Head>) {
    type AppendOpt<Elem: fmt::Debug> = ((Tail, Result<Head>), Result<Elem>);
    fn all(&self) -> bool {
        match self.1 {
            Result::Skipped | Result::Matched(_) => self.0.all(),
            Result::Error => false,
        }
    }
    fn any(&self) -> bool {
        match self.1 {
            Result::Matched(_) => true,
            Result::Skipped | Result::Error => self.0.any(),
        }
    }
    fn append_opt<Elem: fmt::Debug>(self, elem: Result<Elem>) -> Self::AppendOpt<Elem> {
        (self, elem)
    }
    fn format_results(&self) -> String {
        let tail_str = self.0.format_results();
        let current = match &self.1 {
            Result::Skipped => "-".to_string(),
            Result::Error => "✗".to_string(),
            Result::Matched(val) => format!("{:?}", val),
        };
        if tail_str.is_empty() {
            current
        } else {
            format!("{}, {}", tail_str, current)
        }
    }
}

#[derive(Clone)]
pub struct Predicate<T>(pub(crate) fn(&T) -> bool);

pub trait Satisfies<Rhs> {
    fn satisfies(&self, rhs: &Rhs) -> bool;
}

impl<T> Satisfies<T> for T
where
    T: PartialEq,
{
    fn satisfies(&self, rhs: &T) -> bool {
        self == rhs
    }
}

impl<T> Satisfies<T> for Predicate<T> {
    fn satisfies(&self, rhs: &T) -> bool {
        (self.0)(rhs)
    }
}

// Capture consumed actual items into a produced Value.
pub trait Capture<Actual, Value> {
    fn capture(&self, consumed: Vec<Actual>) -> Value;
}

impl Capture<char, String> for () {
    fn capture(&self, consumed: Vec<char>) -> String {
        consumed.into_iter().collect()
    }
}
