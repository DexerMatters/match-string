mod base;
mod ctrl;
mod ext;
mod utils;

pub use crate::base::{Matchable, Matcher};
pub use crate::ext::*;
pub use crate::utils::{Result, ResultHList};

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
