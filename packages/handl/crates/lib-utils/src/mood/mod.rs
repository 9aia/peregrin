#[allow(clippy::module_inception)]
mod mood;

pub use mood::MoodModule;

#[cfg(test)]
pub use mood::{MoodCnd, MoodInd, MoodInt, MoodNeg, MoodNorm, MoodPot};

#[cfg(test)]
mod mood_tests;
