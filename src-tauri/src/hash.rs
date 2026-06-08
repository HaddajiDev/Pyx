pub struct Hasher {
    inner: blake3::Hasher,
}

impl Hasher {
    pub fn new() -> Self {
        Self { inner: blake3::Hasher::new() }
    }

    pub fn update(&mut self, bytes: &[u8]) {
        self.inner.update(bytes);
    }

    pub fn finalize_hex(&self) -> String {
        self.inner.finalize().to_hex().to_string()
    }
}

impl Default for Hasher {
    fn default() -> Self {
        Self::new()
    }
}

pub fn hash_hex(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn incremental_matches_oneshot() {
        let data = b"the quick brown fox";
        let mut h = Hasher::new();
        h.update(&data[..4]);
        h.update(&data[4..]);
        assert_eq!(h.finalize_hex(), hash_hex(data));
    }

    #[test]
    fn known_empty_hash() {
        assert_eq!(
            hash_hex(b""),
            "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262"
        );
    }
}
