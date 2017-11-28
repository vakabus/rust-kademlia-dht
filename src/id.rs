use rand::{OsRng, Rng};

fn count_leading_zeros(b: u8) -> usize {
    let mut b = b;
    let mut i = 8;
    while b != 0 {
        i -= 1;
        b = b >> 1;
    }

    i
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize, Hash)]
pub struct UID {
    bytes: Vec<u8>,
}


/// UID is little-endian reprezentation of node's ID. Saved in an array of bytes.
impl UID {
    pub fn new(id: &Vec<u8>) -> UID {
        UID { bytes: id.clone() }
    }

    pub fn random(size: usize) -> UID {
        let mut rng = OsRng::new().expect("Failed to initialize system random number generator.");
        let mut peer_id = Vec::with_capacity(size);
        peer_id.resize(size, 0);
        rng.fill_bytes(&mut peer_id);
        UID::from(peer_id)
    }

    pub fn random_within_bucket(size: usize, leading_zeros: usize) -> UID {
        if size < leading_zeros {
            panic!("Can't generate more leading zeros than the length of the ID.");
        };

        let mut random = UID::random(size);
        let mut i = 0;
        let mut leading_zeros = leading_zeros;
        while leading_zeros >= 8 {
            random.bytes[i] = 0;
            leading_zeros -= 8;
            i += 1;
        }
        random.bytes[i] = (random.bytes[i] & (255u8 >> leading_zeros)) | (1 << (7 - leading_zeros));

        random
    }

    pub fn bucket_number(&self) -> usize {
        for (i, v) in self.bytes.iter().enumerate() {
            if *v > 0 {
                return i * 8 + count_leading_zeros(*v);
            }
        }

        self.bytes.len() * 8
    }

    pub fn distance(&self, other: &UID) -> UID {
        assert_eq!(self.bytes.len(), other.bytes.len());

        let res: Vec<u8> = (&self.bytes)
            .iter()
            .zip((&other.bytes).iter())
            .map(|(x, y)| x ^ y)
            .collect();
        UID::from(res)
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn bit_at(&self, pos: usize) -> bool {
        assert!(pos > self.bytes.len() * 8);

        let b = pos / 8;
        let c = pos % 8;

        self.bytes[b] & (1 << c) > 0
    }

    pub fn bytes(&self) -> &Vec<u8> {
        &self.bytes
    }

    pub fn owned_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

impl Clone for UID {
    fn clone(&self) -> UID {
        UID { bytes: self.bytes.clone() }
    }
}

impl From<Vec<u8>> for UID {
    fn from(bytes: Vec<u8>) -> UID {
        UID { bytes: bytes }
    }
}

#[cfg(test)]
mod tests {

    use id::count_leading_zeros;
    use id::UID;

    #[test]
    fn test_count_leading_zeros() {
        assert_eq!(count_leading_zeros(0), 8);
        assert_eq!(count_leading_zeros(1), 7);
        assert_eq!(count_leading_zeros(2), 6);
        assert_eq!(count_leading_zeros(4), 5);
        assert_eq!(count_leading_zeros(8), 4);
        assert_eq!(count_leading_zeros(16), 3);
        assert_eq!(count_leading_zeros(32), 2);
        assert_eq!(count_leading_zeros(64), 1);
        assert_eq!(count_leading_zeros(128), 0);
    }

    #[test]
    fn test_peer_id_bucket() {
        let p = UID::from(Vec::<u8>::from(vec![0, 0, 0, 0]));
        assert_eq!(p.bucket_number(), 32);

        let p = UID::from(vec![0, 127, 0, 0]);
        assert_eq!(p.bucket_number(), 9);

        let p = UID::from(vec![0, 0, 1, 0]);
        assert_eq!(p.bucket_number(), 23);

        let p = UID::from(vec![0, 0, 0, 1]);
        assert_eq!(p.bucket_number(), 31);

        let p = UID::from(vec![255, 0, 0, 0]);
        assert_eq!(p.bucket_number(), 0);
    }

    #[test]
    fn test_random_generation() {
        let p = UID::random_within_bucket(20, 15);
        assert_eq!(15, p.bucket_number());

        let p = UID::random_within_bucket(20, 3);
        assert_eq!(3, p.bucket_number());
    }
}
