/// The result of a Resolved guess: Correct if the price moved in the guessed Direction, Wrong otherwise.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Outcome {
    Correct,
    Wrong,
}

impl Outcome {
    /// How the Outcome changes the Score: +1 for Correct, −1 for Wrong.
    #[must_use]
    pub const fn score_change(self) -> i64 {
        match self {
            Self::Correct => 1,
            Self::Wrong => -1,
        }
    }
}
