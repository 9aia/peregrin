use ac_runtime::NonFunctionValue;
use ac_runtime::{Context, Error, Function};

use super::common::{num, require_number};

const PERM: [u8; 256] = [
    151, 160, 137, 91, 90, 15, 131, 13, 201, 95, 96, 53, 194, 233, 7, 225,
    140, 36, 103, 30, 69, 142, 8, 99, 37, 240, 21, 10, 23, 190, 6, 148,
    247, 120, 234, 75, 0, 26, 197, 62, 94, 252, 219, 203, 117, 35, 11, 32,
    57, 177, 33, 88, 237, 149, 56, 87, 174, 20, 125, 136, 171, 168, 68, 175,
    74, 165, 71, 134, 139, 48, 27, 166, 77, 146, 158, 231, 83, 111, 229, 122,
    60, 211, 133, 230, 220, 105, 92, 41, 55, 46, 245, 40, 244, 102, 143, 54,
    65, 25, 63, 161, 1, 216, 80, 73, 209, 76, 132, 187, 208, 89, 18, 169,
    200, 196, 135, 130, 116, 188, 159, 86, 164, 100, 109, 198, 173, 186, 3, 64,
    52, 217, 226, 250, 124, 123, 5, 202, 38, 147, 118, 126, 255, 82, 85, 212,
    207, 206, 59, 227, 47, 16, 58, 17, 182, 189, 28, 42, 223, 183, 170, 213,
    119, 248, 152, 2, 44, 154, 163, 70, 221, 153, 101, 155, 167, 43, 172, 9,
    129, 22, 39, 253, 19, 98, 108, 110, 79, 113, 224, 232, 178, 185, 112, 104,
    218, 246, 97, 228, 251, 34, 242, 193, 238, 210, 144, 12, 191, 179, 162, 241,
    81, 51, 145, 235, 142, 239, 37, 155, 36, 179, 25, 36, 160, 64, 37, 254,
    155, 116, 86, 245, 21, 166, 140, 146, 234, 197, 111, 208, 191, 39, 213, 153,
    88, 214, 105, 92, 41, 55, 46, 245, 40, 244, 102, 143, 54, 65, 25, 63,
];

fn fade(t: f64) -> f64 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + t * (b - a)
}

fn grad(hash: u8, x: f64, y: f64, z: f64) -> f64 {
    let h = hash & 15;
    let u = if h < 8 { x } else { y };
    let v = if h < 4 { y } else if h == 12 || h == 14 { x } else { z };
    (if h & 1 == 0 { u } else { -u }) + (if h & 2 == 0 { v } else { -v })
}

fn perm_index(i: i32) -> usize {
    let idx = i.rem_euclid(256) as usize;
    PERM[idx] as usize
}

fn perlin_1d(x: f64) -> f64 {
    let xi = x.floor() as i32;
    let xf = x - x.floor();

    let aa = PERM[perm_index(xi)] as f64;
    let ba = PERM[perm_index(xi + 1)] as f64;

    let u = fade(xf);
    lerp(aa, ba, u) / 255.0 * 2.0 - 1.0
}

fn perlin_2d(x: f64, y: f64) -> f64 {
    let xi = x.floor() as i32;
    let yi = y.floor() as i32;
    let xf = x - x.floor();
    let yf = y - y.floor();

    let aa = grad(PERM[(perm_index(xi) + perm_index(yi)) % 256], xf, yf, 0.0);
    let ba = grad(PERM[(perm_index(xi + 1) + perm_index(yi)) % 256], xf - 1.0, yf, 0.0);
    let ab = grad(PERM[(perm_index(xi) + perm_index(yi + 1)) % 256], xf, yf - 1.0, 0.0);
    let bb = grad(PERM[(perm_index(xi + 1) + perm_index(yi + 1)) % 256], xf - 1.0, yf - 1.0, 0.0);

    let u = fade(xf);
    let v = fade(yf);
    lerp(lerp(aa, ba, u), lerp(ab, bb, u), v)
}

fn perlin_3d(x: f64, y: f64, z: f64) -> f64 {
    let xi = x.floor() as i32;
    let yi = y.floor() as i32;
    let zi = z.floor() as i32;
    let xf = x - x.floor();
    let yf = y - y.floor();
    let zf = z - z.floor();

    let p = |i: i32| perm_index(i);

    let aaa = grad(PERM[(p(xi) + p(yi) + p(zi)) % 256], xf, yf, zf);
    let baa = grad(PERM[(p(xi + 1) + p(yi) + p(zi)) % 256], xf - 1.0, yf, zf);
    let aba = grad(PERM[(p(xi) + p(yi + 1) + p(zi)) % 256], xf, yf - 1.0, zf);
    let bba = grad(PERM[(p(xi + 1) + p(yi + 1) + p(zi)) % 256], xf - 1.0, yf - 1.0, zf);
    let aab = grad(PERM[(p(xi) + p(yi) + p(zi + 1)) % 256], xf, yf, zf - 1.0);
    let bab = grad(PERM[(p(xi + 1) + p(yi) + p(zi + 1)) % 256], xf - 1.0, yf, zf - 1.0);
    let abb = grad(PERM[(p(xi) + p(yi + 1) + p(zi + 1)) % 256], xf, yf - 1.0, zf - 1.0);
    let bbb = grad(PERM[(p(xi + 1) + p(yi + 1) + p(zi + 1)) % 256], xf - 1.0, yf - 1.0, zf - 1.0);

    let u = fade(xf);
    let v = fade(yf);
    let w = fade(zf);
    let x1 = lerp(aaa, baa, u);
    let x2 = lerp(aba, bba, u);
    let y1 = lerp(x1, x2, v);
    let x3 = lerp(aab, bab, u);
    let x4 = lerp(abb, bbb, u);
    let y2 = lerp(x3, x4, v);
    lerp(y1, y2, w)
}

/// `rand.perlin_1d(x: number) → number`
///
/// Returns 1D Perlin noise value at coordinate `x`. Output is
/// approximately in the range `[-1, 1]`.
///
/// Uses a fixed permutation table (Ken Perlin's original) for
/// deterministic results across runs.
///
/// Example:
/// - `rand.perlin_1d(0.5)` → noise at x=0.5
#[derive(Clone, Copy)]
pub struct Perlin1D;

impl Function for Perlin1D {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut dyn Context) -> Result<NonFunctionValue, Error> {
        let x = require_number(args, 0)?;
        Ok(num(perlin_1d(x)))
    }
}

/// `rand.perlin_2d(x: number, y: number) → number`
///
/// Returns 2D Perlin noise value at coordinates `(x, y)`. Output is
/// approximately in the range `[-1, 1]`.
///
/// Uses a fixed permutation table (Ken Perlin's original) for
/// deterministic results across runs.
///
/// Example:
/// - `rand.perlin_2d(1.0, 2.0)` → noise at (1, 2)
#[derive(Clone, Copy)]
pub struct Perlin2D;

impl Function for Perlin2D {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut dyn Context) -> Result<NonFunctionValue, Error> {
        let x = require_number(args, 0)?;
        let y = require_number(args, 1)?;
        Ok(num(perlin_2d(x, y)))
    }
}

/// `rand.perlin_3d(x: number, y: number, z: number) → number`
///
/// Returns 3D Perlin noise value at coordinates `(x, y, z)`. Output is
/// approximately in the range `[-1, 1]`.
///
/// Uses a fixed permutation table (Ken Perlin's original) for
/// deterministic results across runs.
///
/// Example:
/// - `rand.perlin_3d(0.1, 0.2, 0.3)` → noise at (0.1, 0.2, 0.3)
#[derive(Clone, Copy)]
pub struct Perlin3D;

impl Function for Perlin3D {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut dyn Context) -> Result<NonFunctionValue, Error> {
        let x = require_number(args, 0)?;
        let y = require_number(args, 1)?;
        let z = require_number(args, 2)?;
        Ok(num(perlin_3d(x, y, z)))
    }
}

// ── Simplex noise ────────────────────────────────────────────────

const F2: f64 = 0.3660254037844386; // 0.5 * (sqrt(3) - 1)
const G2: f64 = 0.21132486540518713; // (sqrt(3) - 1) / 2
const F3: f64 = 1.0 / 3.0;
const G3: f64 = 1.0 / 6.0;

const GRAD3: [(f64, f64, f64); 12] = [
    (1.0, 1.0, 0.0), (-1.0, 1.0, 0.0), (1.0, -1.0, 0.0), (-1.0, -1.0, 0.0),
    (1.0, 0.0, 1.0), (-1.0, 0.0, 1.0), (1.0, 0.0, -1.0), (-1.0, 0.0, -1.0),
    (0.0, 1.0, 1.0), (0.0, -1.0, 1.0), (0.0, 1.0, -1.0), (0.0, -1.0, -1.0),
];

fn grad3(hash: u8, x: f64, y: f64, z: f64) -> f64 {
    let g = GRAD3[(hash % 12) as usize];
    g.0 * x + g.1 * y + g.2 * z
}

fn simplex_2d(x: f64, y: f64) -> f64 {
    let s = (x + y) * F2;
    let i = (x + s).floor() as i32;
    let j = (y + s).floor() as i32;
    let t = (i + j) as f64 * G2;
    let x0 = x - (i as f64 - t);
    let y0 = y - (j as f64 - t);

    let (i1, j1) = if x0 > y0 { (1, 0) } else { (0, 1) };

    let x1 = x0 - i1 as f64 + G2;
    let y1 = y0 - j1 as f64 + G2;
    let x2 = x0 - 1.0 + 2.0 * G2;
    let y2 = y0 - 1.0 + 2.0 * G2;

    let ii = i.rem_euclid(256);
    let jj = j.rem_euclid(256);
    let gi0 = PERM[(PERM[ii as usize] as usize + jj as usize) % 256];
    let gi1 = PERM[(PERM[(ii + i1).rem_euclid(256) as usize] as usize + (jj + j1).rem_euclid(256) as usize) % 256];
    let gi2 = PERM[(PERM[(ii + 1).rem_euclid(256) as usize] as usize + (jj + 1).rem_euclid(256) as usize) % 256];

    let mut n0 = 0.0;
    let mut n1 = 0.0;
    let mut n2 = 0.0;

    let t0 = 0.5 - x0 * x0 - y0 * y0;
    if t0 >= 0.0 {
        let t0_sq = t0 * t0;
        n0 = t0_sq * t0_sq * grad3(gi0, x0, y0, 0.0);
    }

    let t1 = 0.5 - x1 * x1 - y1 * y1;
    if t1 >= 0.0 {
        let t1_sq = t1 * t1;
        n1 = t1_sq * t1_sq * grad3(gi1, x1, y1, 0.0);
    }

    let t2 = 0.5 - x2 * x2 - y2 * y2;
    if t2 >= 0.0 {
        let t2_sq = t2 * t2;
        n2 = t2_sq * t2_sq * grad3(gi2, x2, y2, 0.0);
    }

    70.0 * (n0 + n1 + n2)
}

fn simplex_3d(x: f64, y: f64, z: f64) -> f64 {
    let s = (x + y + z) * F3;
    let i = (x + s).floor() as i32;
    let j = (y + s).floor() as i32;
    let k = (z + s).floor() as i32;
    let t = (i + j + k) as f64 * G3;
    let x0 = x - (i as f64 - t);
    let y0 = y - (j as f64 - t);
    let z0 = z - (k as f64 - t);

    let (i1, j1, k1, i2, j2, k2);
    if x0 >= y0 {
        if y0 >= z0 {
            i1 = 1; j1 = 0; k1 = 0; i2 = 1; j2 = 1; k2 = 0;
        } else if x0 >= z0 {
            i1 = 1; j1 = 0; k1 = 0; i2 = 1; j2 = 0; k2 = 1;
        } else {
            i1 = 0; j1 = 0; k1 = 1; i2 = 1; j2 = 0; k2 = 1;
        }
    } else {
        if y0 < z0 {
            i1 = 0; j1 = 0; k1 = 1; i2 = 0; j2 = 1; k2 = 1;
        } else if x0 < z0 {
            i1 = 0; j1 = 1; k1 = 0; i2 = 0; j2 = 1; k2 = 1;
        } else {
            i1 = 0; j1 = 1; k1 = 0; i2 = 1; j2 = 1; k2 = 0;
        }
    }

    let x1 = x0 - i1 as f64 + G3;
    let y1 = y0 - j1 as f64 + G3;
    let z1 = z0 - k1 as f64 + G3;
    let x2 = x0 - i2 as f64 + 2.0 * G3;
    let y2 = y0 - j2 as f64 + 2.0 * G3;
    let z2 = z0 - k2 as f64 + 2.0 * G3;
    let x3 = x0 - 1.0 + 3.0 * G3;
    let y3 = y0 - 1.0 + 3.0 * G3;
    let z3 = z0 - 1.0 + 3.0 * G3;

    let ii = i.rem_euclid(256) as usize;
    let jj = j.rem_euclid(256) as usize;
    let kk = k.rem_euclid(256) as usize;

    let gi0 = PERM[(PERM[ii] as usize + jj + kk) % 256];
    let gi1 = PERM[(PERM[(ii + i1 as usize) % 256] as usize + (jj + j1 as usize) % 256 + (kk + k1 as usize) % 256) % 256];
    let gi2 = PERM[(PERM[(ii + i2 as usize) % 256] as usize + (jj + j2 as usize) % 256 + (kk + k2 as usize) % 256) % 256];
    let gi3 = PERM[(PERM[(ii + 1) % 256] as usize + (jj + 1) % 256 + (kk + 1) % 256) % 256];

    let mut n0 = 0.0;
    let mut n1 = 0.0;
    let mut n2 = 0.0;
    let mut n3 = 0.0;

    let t0 = 0.6 - x0 * x0 - y0 * y0 - z0 * z0;
    if t0 >= 0.0 {
        let t0_sq = t0 * t0;
        n0 = t0_sq * t0_sq * grad3(gi0, x0, y0, z0);
    }

    let t1 = 0.6 - x1 * x1 - y1 * y1 - z1 * z1;
    if t1 >= 0.0 {
        let t1_sq = t1 * t1;
        n1 = t1_sq * t1_sq * grad3(gi1, x1, y1, z1);
    }

    let t2 = 0.6 - x2 * x2 - y2 * y2 - z2 * z2;
    if t2 >= 0.0 {
        let t2_sq = t2 * t2;
        n2 = t2_sq * t2_sq * grad3(gi2, x2, y2, z2);
    }

    let t3 = 0.6 - x3 * x3 - y3 * y3 - z3 * z3;
    if t3 >= 0.0 {
        let t3_sq = t3 * t3;
        n3 = t3_sq * t3_sq * grad3(gi3, x3, y3, z3);
    }

    32.0 * (n0 + n1 + n2 + n3)
}

/// `rand.simplex_2d(x: number, y: number) → number`
///
/// Returns 2D Simplex noise value at coordinates `(x, y)`. Output is
/// approximately in the range `[-1, 1]`.
///
/// Uses a fixed permutation table for deterministic results across runs.
/// Simplex noise has fewer directional artifacts than Perlin noise.
///
/// Example:
/// - `rand.simplex_2d(1.0, 2.0)` → noise at (1, 2)
#[derive(Clone, Copy)]
pub struct Simplex2D;

impl Function for Simplex2D {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut dyn Context) -> Result<NonFunctionValue, Error> {
        let x = require_number(args, 0)?;
        let y = require_number(args, 1)?;
        Ok(num(simplex_2d(x, y)))
    }
}

/// `rand.simplex_3d(x: number, y: number, z: number) → number`
///
/// Returns 3D Simplex noise value at coordinates `(x, y, z)`. Output is
/// approximately in the range `[-1, 1]`.
///
/// Uses a fixed permutation table for deterministic results across runs.
/// Simplex noise has fewer directional artifacts than Perlin noise.
///
/// Example:
/// - `rand.simplex_3d(0.1, 0.2, 0.3)` → noise at (0.1, 0.2, 0.3)
#[derive(Clone, Copy)]
pub struct Simplex3D;

impl Function for Simplex3D {
    fn call(&self, args: &[NonFunctionValue], _ctx: &mut dyn Context) -> Result<NonFunctionValue, Error> {
        let x = require_number(args, 0)?;
        let y = require_number(args, 1)?;
        let z = require_number(args, 2)?;
        Ok(num(simplex_3d(x, y, z)))
    }
}
