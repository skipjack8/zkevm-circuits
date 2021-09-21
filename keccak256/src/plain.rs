type State = [[u64; 5]; 5];

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

struct Keccak {
    state: State,
    sponge: Sponge,
}

impl Keccak {
    pub fn new() -> Keccak {
        let security_level = (1088, 512);

        Keccak {
            state: [[0; 5]; 5],
            // rate & capacity in bytes
            sponge: Sponge::new(security_level.0 / 8, security_level.1 / 8),
        }
    }

    pub fn update(&mut self, input: &[u8]) {
        let padding_total = self.sponge.rate - (input.len() % self.sponge.rate);
        let mut padding: Vec<u8>;

        if padding_total == 1 {
            padding = vec![0x81];
        } else {
            padding = vec![];
            padding.push(0x01);

            for _ in 0..(padding_total - 2) {
                padding.push(0x00);
            }

            padding.push(0x80);
        }

        let padded_input: &[u8] = &[input, &padding].concat();
        self.sponge.absorb(&mut self.state, padded_input);
    }

    /// Returns keccak hash based on current state
    pub fn digest(&mut self) -> Vec<u8> {
        self.sponge.squeeze(&mut self.state)
    }
}

struct KeccakF {}

impl KeccakF {
    pub fn new() -> KeccakF {
        KeccakF {}
    }

    pub fn permutations(&self, a: &mut State) {
        for i in 0..PERMUTATION {
            *a = KeccakF::round_b(*a, ROUND_CONSTANTS[i]);
        }
    }

    fn round_b(a: State, rc: u64) -> State {
        let s1 = KeccakF::theta(a);
        let s2 = KeccakF::rho(s1);
        let s3 = KeccakF::pi(s2);
        let s4 = KeccakF::xi(s3);
        let s5 = KeccakF::iota(s4, rc);
        s5
    }

    fn theta(a: State) -> State {
        let mut c: [u64; 5] = [0; 5];
        let mut out: State = [[0; 5]; 5];

        for x in 0..5 {
            c[x] = a[x][0] ^ a[x][1] ^ a[x][2] ^ a[x][3] ^ a[x][4];
        }

        for x in 0..5 {
            for y in 0..5 {
                out[x][y] =
                    a[x][y] ^ c[(x + 4) % 5] ^ c[(x + 1) % 5].rotate_left(1);
            }
        }
        out
    }

    fn rho(a: State) -> State {
        let mut out: State = [[0; 5]; 5];
        for x in 0..5 {
            for y in 0..5 {
                out[x][y] = a[x][y].rotate_left(ROTATION_CONSTANTS[x][y]);
            }
        }
        out
    }

    fn pi(a: State) -> State {
        let mut out: State = [[0; 5]; 5];

        for x in 0..5 {
            for y in 0..5 {
                out[y][(2 * x + 3 * y) % 5] = a[x][y];
            }
        }
        out
    }

    fn xi(a: State) -> State {
        let mut out: State = [[0; 5]; 5];
        for x in 0..5 {
            for y in 0..5 {
                out[x][y] = a[x][y] ^ (!a[(x + 1) % 5][y] & a[(x + 2) % 5][y]);
            }
        }
        out
    }

    fn iota(a: State, rc: u64) -> State {
        let mut out = a;
        out[0][0] ^= rc;
        out
    }
}

struct Sponge {
    rate: usize,
    capacity: usize,
    keccak_f: KeccakF,
}

impl Sponge {
    pub fn new(rate: usize, capacity: usize) -> Sponge {
        Sponge {
            rate: rate,
            capacity: capacity,
            keccak_f: KeccakF::new(),
        }
    }

    pub fn absorb(&self, mut state: &mut State, message: &[u8]) {
        assert!(
            message.len() % self.rate == 0,
            "Message is not divisible entirely by bytes rate"
        );

        let chunks_total = message.len() / self.rate;

        let words: Vec<u64> = Sponge::bits_to_u64_words_le(message);

        for chunk_i in 0..chunks_total {
            let chunk_offset: usize = chunk_i * (self.rate / 8);
            let mut x = 0;
            let mut y = 0;
            for i in 0..(self.rate / 8) {
                let word = words[chunk_offset + i];
                state[x][y] ^= word;
                if x < 5 - 1 {
                    x += 1;
                } else {
                    y += 1;
                    x = 0;
                }
            }
            self.keccak_f.permutations(&mut state);
        }
    }

    pub fn squeeze(&self, state: &mut State) -> Vec<u8> {
        let mut output: Vec<u8> = vec![];

        let output_len: usize = self.capacity / 2;
        let elems_total: usize = output_len / 8;
        let mut counter: usize = 0;

        'outer: for i in 0..5 {
            for j in 0..5 {
                output.append(&mut state[j][i].to_le_bytes().to_vec());
                if counter == elems_total {
                    break 'outer;
                }
                counter += 1;
            }
        }

        output.resize(output_len, 0);
        output
    }

    fn bits_to_u64_words_le(message: &[u8]) -> Vec<u64> {
        let words_total = message.len() / 8;
        let mut words: Vec<u64> = vec![0; words_total];

        for i in 0..words_total {
            let mut word_bits: [u8; 8] = Default::default();
            word_bits.copy_from_slice(&message[i * 8..i * 8 + 8]);
            words[i] = u64::from_le_bytes(word_bits);
        }
        words
    }
}
#[cfg(test)]
fn keccak256(msg: &[u8]) -> Vec<u8> {
    let mut keccak = Keccak::new();
    keccak.update(msg);
    keccak.digest()
}

#[test]
fn test_empty_input() {
    let output = [
        197, 210, 70, 1, 134, 247, 35, 60, 146, 126, 125, 178, 220, 199, 3,
        192, 229, 0, 182, 83, 202, 130, 39, 59, 123, 250, 216, 4, 93, 133, 164,
        112,
    ];
    assert_eq!(keccak256(&[]), output);
}

#[test]
fn test_short_input() {
    let output = [
        56, 209, 138, 203, 103, 210, 92, 139, 185, 148, 39, 100, 182, 47, 24,
        225, 112, 84, 246, 106, 129, 123, 212, 41, 84, 35, 173, 249, 237, 152,
        135, 62,
    ];
    assert_eq!(keccak256(&[102, 111, 111, 98, 97, 114]), output);
}

#[test]
fn test_long_input() {
    let input = [
        65, 108, 105, 99, 101, 32, 119, 97, 115, 32, 98, 101, 103, 105, 110,
        110, 105, 110, 103, 32, 116, 111, 32, 103, 101, 116, 32, 118, 101, 114,
        121, 32, 116, 105, 114, 101, 100, 32, 111, 102, 32, 115, 105, 116, 116,
        105, 110, 103, 32, 98, 121, 32, 104, 101, 114, 32, 115, 105, 115, 116,
        101, 114, 32, 111, 110, 32, 116, 104, 101, 32, 98, 97, 110, 107, 44,
        32, 97, 110, 100, 32, 111, 102, 32, 104, 97, 118, 105, 110, 103, 32,
        110, 111, 116, 104, 105, 110, 103, 32, 116, 111, 32, 100, 111, 58, 32,
        111, 110, 99, 101, 32, 111, 114, 32, 116, 119, 105, 99, 101, 32, 115,
        104, 101, 32, 104, 97, 100, 32, 112, 101, 101, 112, 101, 100, 32, 105,
        110, 116, 111, 32, 116, 104, 101, 32, 98, 111, 111, 107, 32, 104, 101,
        114, 32, 115, 105, 115, 116, 101, 114, 32, 119, 97, 115, 32, 114, 101,
        97, 100, 105, 110, 103, 44, 32, 98, 117, 116, 32, 105, 116, 32, 104,
        97, 100, 32, 110, 111, 32, 112, 105, 99, 116, 117, 114, 101, 115, 32,
        111, 114, 32, 99, 111, 110, 118, 101, 114, 115, 97, 116, 105, 111, 110,
        115, 32, 105, 110, 32, 105, 116, 44, 32, 97, 110, 100, 32, 119, 104,
        97, 116, 32, 105, 115, 32, 116, 104, 101, 32, 117, 115, 101, 32, 111,
        102, 32, 97, 32, 98, 111, 111, 107, 44, 32, 116, 104, 111, 117, 103,
        104, 116, 32, 65, 108, 105, 99, 101, 32, 119, 105, 116, 104, 111, 117,
        116, 32, 112, 105, 99, 116, 117, 114, 101, 115, 32, 111, 114, 32, 99,
        111, 110, 118, 101, 114, 115, 97, 116, 105, 111, 110, 115, 63,
    ];
    let output = [
        60, 227, 142, 8, 143, 135, 108, 85, 13, 254, 190, 58, 30, 106, 153,
        194, 188, 6, 208, 49, 16, 102, 150, 120, 100, 130, 224, 177, 64, 98,
        53, 252,
    ];
    assert_eq!(keccak256(&input), output);
}