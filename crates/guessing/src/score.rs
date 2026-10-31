/// A Player's running total: it starts at 0 and can go below it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Score(i64);

impl Score {
    #[must_use]
    pub const fn new(value: i64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn value(self) -> i64 {
        self.0
    }
}
