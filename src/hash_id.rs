use std::ops::BitXor;

#[derive(PartialEq, Clone, Copy)]
pub struct HashId(pub [u32; 8]);

impl BitXor for HashId {
    type Output = [u32; 8];
    fn bitxor(self, rhs: Self) -> Self::Output {
        let mut xor: [u32; 8] = [0; 8];
        for chunk in 0..8 {
            xor[chunk] = self.0[chunk] ^ rhs.0[chunk];
        }
        xor
    }
}

impl HashId {
    pub fn leading_zeros(&self, other: &HashId) -> usize {
        let xor = *self ^ *other;
        for chunk in 0..8 {
            match xor[chunk].leading_zeros() {
                32 => continue, // all zeros, chunks are the same, continue
                num_zeros => return (chunk * 32) + num_zeros as usize,
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
        let base_id = HashId([5324235, 0, 0, 0, 0, 0, 0, 0]);
        let other_id = HashId([3242578, 0, 0, 0, 0, 0, 0, 0]);

        let l_0 = base_id.leading_zeros(&other_id);
        assert_eq!(l_0, 9);
        assert_ne!(l_0, 256);

        let l_0 = base_id.leading_zeros(&base_id);
        assert_eq!(l_0, 256);
        assert_ne!(l_0, 9);
    }
}
