use hash::DHTHasher;
use crypto::digest::Digest;
use crypto::sha1::Sha1;

pub struct SHA1Hasher;

impl SHA1Hasher {
    pub fn new() -> SHA1Hasher {
        SHA1Hasher {}
    }
}

impl DHTHasher for SHA1Hasher {
    fn hash(data: &[u8]) -> Vec<u8> {
        let mut h = Sha1::new();
        h.input(data);
        let mut hash = Vec::with_capacity(h.output_bytes());
        hash.resize(h.output_bytes(), 0);
        h.result(&mut hash);
        hash
    }

    fn get_hash_bytes_count() -> usize {
        20
    }
}

#[cfg(test)]
mod tests {

    use hash::sha1::SHA1Hasher;
    use DHTHasher;

    fn to_hex_string(bytes: Vec<u8>) -> String {
        let strs: Vec<String> = bytes.iter().map(|b| format!("{:02x}", b)).collect();
        strs.join("")
    }

    #[test]
    fn valide_sha1_hash() {
        let data = b"password";
        let hash: Vec<u8> = SHA1Hasher::hash(&data[..]);
        assert_eq!(
            to_hex_string(hash),
            "5baa61e4c9b93f3f0682250b6cf8331b7ee68fd8"
        );
    }

    #[test]
    fn validate_sha1_empty_hash() {
        let data = b"";
        let hash: Vec<u8> = SHA1Hasher::hash(&data[..]);
        assert_eq!(
            to_hex_string(hash),
            "da39a3ee5e6b4b0d3255bfef95601890afd80709"
        );
    }
}
