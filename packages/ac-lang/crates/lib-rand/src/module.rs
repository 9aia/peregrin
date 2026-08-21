use ac_runtime_core::Module;
use ac_vm::Function;

use super::distribution::Gauss;
use super::generation::{Bytes, String_, Uuid};
use super::noise::{Perlin1D, Perlin2D, Perlin3D};
use super::numbers::{Bool, Float, Int};
use super::sampling::{Choice, Pick, Sample, Shuffle, WeightedPick};
use super::seeded::{
    SeededBool, SeededChoice, SeededFloat, SeededGauss, SeededInt, SeededPick,
    SeededSample, SeededShuffle, SeededString,
};

/// The `rand` module — random number generation and sampling.
///
/// Provides functions callable as `rand.int(max)`, `rand.int(min, max)`,
/// `rand.float()`, `rand.float(max)`, `rand.float(min, max)`,
/// `rand.bool()`, `rand.pick(array)`, `rand.choice(array, count)`, `rand.sample(array, count)`,
/// `rand.shuffle(array)`, `rand.weighted_pick(items, weights)`,
/// `rand.string(length, charset?)`, `rand.bytes(length)`,
/// `rand.uuid()`, `rand.gauss(mean, stddev)`,
/// `rand.perlin_1d(x)`, `rand.perlin_2d(x, y)`, `rand.perlin_3d(x, y, z)`,
/// `rand.seeded_int(seed, min, max)`, `rand.seeded_float(seed, min, max)`,
/// `rand.seeded_bool(seed)`, `rand.seeded_pick(seed, array)`,
/// `rand.seeded_choice(seed, array, count)`, `rand.seeded_sample(seed, array, count)`,
/// `rand.seeded_shuffle(seed, array)`, `rand.seeded_string(seed, length, charset?)`,
/// `rand.seeded_gauss(seed, mean, stddev)` in AC expressions.
///
/// All non-seeded functions use the system RNG via the `rand` crate.
/// Each call is independent — there is no shared state between calls.
/// Seeded functions are deterministic — same seed + same args always
/// produce the same result.
/// Perlin noise functions are deterministic (fixed permutation table).
#[derive(Clone, Copy)]
pub struct RandModule;

impl Module for RandModule {
    fn name(&self) -> &str {
        "rand"
    }

    fn function(&self, name: &str) -> Option<&dyn Function> {
        match name {
            "int" => Some(&Int),
            "float" => Some(&Float),
            "bool" => Some(&Bool),
            "pick" => Some(&Pick),
            "choice" => Some(&Choice),
            "sample" => Some(&Sample),
            "shuffle" => Some(&Shuffle),
            "weighted_pick" => Some(&WeightedPick),
            "string" => Some(&String_),
            "bytes" => Some(&Bytes),
            "uuid" => Some(&Uuid),
            "gauss" => Some(&Gauss),
            "perlin_1d" => Some(&Perlin1D),
            "perlin_2d" => Some(&Perlin2D),
            "perlin_3d" => Some(&Perlin3D),
            "seeded_int" => Some(&SeededInt),
            "seeded_float" => Some(&SeededFloat),
            "seeded_bool" => Some(&SeededBool),
            "seeded_pick" => Some(&SeededPick),
            "seeded_choice" => Some(&SeededChoice),
            "seeded_sample" => Some(&SeededSample),
            "seeded_shuffle" => Some(&SeededShuffle),
            "seeded_string" => Some(&SeededString),
            "seeded_gauss" => Some(&SeededGauss),
            _ => None,
        }
    }
}
