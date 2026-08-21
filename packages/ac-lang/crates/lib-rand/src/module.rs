use ac_runtime_core::Module;
use ac_vm::Function;

use super::random::{Bool, Bytes, Float, Gauss, Int, Perlin1D, Perlin2D, Perlin3D, Pick, Sample, Shuffle, String_, Uuid, WeightedPick};

/// The `rand` module — random number generation and sampling.
///
/// Provides functions callable as `rand.int(max)`, `rand.int(min, max)`,
/// `rand.float()`, `rand.float(max)`, `rand.float(min, max)`,
/// `rand.bool()`, `rand.pick(array)`, `rand.sample(array, count)`,
/// `rand.shuffle(array)`, `rand.weighted_pick(items, weights)`,
/// `rand.string(length, charset?)`, `rand.bytes(length)`,
/// `rand.uuid()`, `rand.gauss(mean, stddev)`,
/// `rand.perlin_1d(x)`, `rand.perlin_2d(x, y)`, `rand.perlin_3d(x, y, z)` in AC expressions.
///
/// All functions use the system RNG via the `rand` crate.
/// Each call is independent — there is no shared state between calls.
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
            _ => None,
        }
    }
}
