use std::ops::BitXor;

#[derive(PartialEq, Clone, Copy)]
pub struct HashId(pub [u8; 32]);

impl BitXor for HashId {
    type Output = [u8; 32];
    fn bitxor(self, rhs: Self) -> Self::Output {
        let mut xor: [u8; 32] = [0; 32];
        for chunk in 0..32 {
            xor[chunk] = self.0[chunk] ^ rhs.0[chunk];
        }
        xor
    }
}

impl AsRef<[u8]> for HashId {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl Into<Vec<u8>> for HashId {
    fn into(self) -> Vec<u8> {
        Vec::from(self.0)
    }
}

impl From<[u8; 32]> for HashId {
    fn from(bytes: [u8; 32]) -> Self {
        HashId(bytes)
    }
}

impl HashId {
    pub fn leading_zeros(&self, other: &HashId) -> usize {
        let xor = *self ^ *other;
        for (i, chunk) in xor.iter().enumerate() {
            match chunk.leading_zeros() {
                8 => continue, // all zeros, chunks are the same, continue
                num_zeros => return (i * 8) + num_zeros as usize,
            }
        }
        256 // all bits are equivalent
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test() {
        let base_id = HashId([
            27, 64, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0,
        ]);
        let other_id = HashId([
            27, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]);

        let zeroes = base_id.leading_zeros(&other_id);
        assert_eq!(zeroes, 9);
        assert_ne!(zeroes, 256);

        let zeroes = base_id.leading_zeros(&base_id);
        assert_eq!(zeroes, 256);
        assert_ne!(zeroes, 9);
    }
}
