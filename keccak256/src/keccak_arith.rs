use num_bigint::BigUint;
use num_traits::{One, Zero};
use std::ops::{Index, IndexMut};

const B13: u64 = 13;
const B9: u64 = 9;

type Lane13 = BigUint;
type Lane9 = BigUint;

struct State {
    xy: Vec<BigUint>,
}
impl Default for State {
    fn default() -> Self {
        let mut xy: Vec<BigUint> = Vec::with_capacity(25);
        for _ in 0..25 {
            xy.push(Zero::zero());
        }
        Self { xy }
    }
}

impl Index<(usize, usize)> for State {
    type Output = BigUint;
    fn index(&self, xy: (usize, usize)) -> &Self::Output {
        assert!(xy.0 < 5);
        assert!(xy.1 < 5);

        &self.xy[xy.0 * 5 + xy.1]
    }
}

impl IndexMut<(usize, usize)> for State {
    fn index_mut(&mut self, xy: (usize, usize)) -> &mut Self::Output {
        assert!(xy.0 < 5);
        assert!(xy.1 < 5);

        &mut self.xy[xy.0 * 5 + xy.1]
    }
}

impl Clone for State {
    fn clone(&self) -> State {
        let xy = self.xy.clone();
        State { xy }
    }
}

const PERMUTATION: usize = 24;

static ROUND_CONSTANTS: [u64; PERMUTATION] = [
    0x0000000000000001,
    0x0000000000008082,
    0x800000000000808A,
    0x8000000080008000,
    0x000000000000808B,
    0x0000000080000001,
    0x8000000080008081,
    0x8000000000008009,
    0x000000000000008A,
    0x0000000000000088,
    0x0000000080008009,
    0x000000008000000A,
    0x000000008000808B,
    0x800000000000008B,
    0x8000000000008089,
    0x8000000000008003,
    0x8000000000008002,
    0x8000000000000080,
    0x000000000000800A,
    0x800000008000000A,
    0x8000000080008081,
    0x8000000000008080,
    0x0000000080000001,
    0x8000000080008008,
];

static ROTATION_CONSTANTS: [[u32; 5]; 5] = [
    [0, 36, 3, 41, 18],
    [1, 44, 10, 45, 2],
    [62, 6, 43, 15, 61],
    [28, 55, 25, 21, 56],
    [27, 20, 39, 8, 14],
];

fn convert_b2_to_b13(a: u64) -> Lane13 {
    let mut lane13: BigUint = Zero::zero();
    for i in 0..64 {
        let bit = a >> i;
        lane13 += bit * B13.pow(i);
    }
    lane13.into()
}

fn convert_b2_to_b9(a: u64) -> Lane9 {
    let mut lane9: BigUint = Zero::zero();
    for i in 0..64 {
        let bit = a >> i;
        lane9 += bit * B9.pow(i);
    }
    lane9.into()
}

fn convert_b13_coef(x: u64) -> u64 {
    assert!(x < 13);
    x & 1
}
fn convert_b9_coef(x: u64) -> u64 {
    assert!(x < 9);
    let bit_table: [u64; 9] = [0, 0, 1, 1, 0, 0, 1, 1, 0];
    bit_table[x as usize]
}

fn convert_b13_lane_to_b9(x: Lane13, rot: u32) -> Lane9 {
    let mut base = B9.pow(rot.into());
    let mut special_chunk = Zero::zero();
    let mut raw = x;
    let mut acc: Lane9 = Zero::zero();

    for i in 0..65 {
        let remainder: u64 = (raw.clone() % B13).to_u64_digits()[0];
        if i == 0 || i == 64 {
            special_chunk += remainder;
        } else {
            acc += convert_b13_coef(remainder) * base;
        }
        raw /= B13;
        base *= B9;
        if i == 64 - rot {
            base = 1;
        }

        acc += convert_b13_coef(special_chunk) * B9.pow(rot.into());
    }
    acc
}

fn convert_b9_lane_to_b13(x: Lane9) -> Lane13 {
    let mut base: Lane13 = One::one();
    let mut raw = x;
    let mut acc: Lane13 = Zero::zero();

    for _ in 0..64 {
        let remainder: u64 = (raw.clone() % B9).to_u64_digits()[0];
        acc += convert_b9_coef(remainder) * base.clone();
        raw /= B9;
        base *= B13;
    }
    acc
}

struct KeccakFArith {}

impl KeccakFArith {
    pub fn new() -> KeccakFArith {
        KeccakFArith {}
    }

    pub fn permutations(&self, a: &State) {
        let mut in_b13 = State::default();
        for x in 0..5 {
            for y in 0..5 {
                in_b13[(x, y)] = a[(x, y)].clone();
            }
        }

        for i in 0..PERMUTATION {
            let out_b9 =
                KeccakFArith::round_b(in_b13.clone(), ROUND_CONSTANTS[i]);
            for x in 0..5 {
                for y in 0..5 {
                    in_b13[(x, y)] =
                        convert_b9_lane_to_b13(out_b9[(x, y)].clone());
                }
            }
        }
    }

    fn round_b(a: State, rc: u64) -> State {
        let s1 = KeccakFArith::theta(&a);
        let s2 = KeccakFArith::rho(&s1);

        let s3 = KeccakFArith::pi(&s2);
        let s4 = KeccakFArith::xi(&s3);
        let s5 = KeccakFArith::iota(&s4, rc);
        s5
    }

    fn theta(a: &State) -> State {
        let mut c: Vec<Lane13> = Vec::with_capacity(5);
        let mut out = State::default();

        for x in 0..5 {
            c[x] =
                &a[(x, 0)] + &a[(x, 1)] + &a[(x, 2)] + &a[(x, 3)] + &a[(x, 4)];
        }

        for x in 0..5 {
            for y in 0..5 {
                out[(x, y)] =
                    &a[(x, y)] + &c[(x + 4) % 5] + &c[(x + 1) % 5] * B13;
            }
        }
        out
    }

    fn rho(a: &State) -> State {
        let mut out = State::default();
        for x in 0..5 {
            for y in 0..5 {
                out[(x, y)] = convert_b13_lane_to_b9(
                    a[(x, y)].clone(),
                    ROTATION_CONSTANTS[x][y],
                );
            }
        }
        out
    }

    fn pi(a: &State) -> State {
        let mut out = State::default();
        for x in 0..5 {
            for y in 0..5 {
                out[(y, (2 * x + 3 * y) % 5)] = a[(x, y)].clone();
            }
        }
        out
    }

    fn xi(a: &State) -> State {
        let mut out = State::default();
        let two = BigUint::from(2 as u8);
        let three = BigUint::from(3 as u8);
        for x in 0..5 {
            for y in 0..5 {
                out[(x, y)] = two.clone() * a[(x, y)].clone()
                    + a[((x + 1) % 5, y)].clone()
                    + three.clone() * a[((x + 2) % 5, y)].clone();
            }
        }
        out
    }

    fn iota(a: &State, rc: u64) -> State {
        let mut out = a.clone();
        out[(0, 0)] += BigUint::from(2 as u8) * convert_b2_to_b9(rc);
        out
    }
}