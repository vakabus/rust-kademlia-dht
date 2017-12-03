//! Generic hash interface.

pub mod sha1;

pub trait DHTHasher {
    /// Hash any given data to an array of constant size.
    fn hash(data: &[u8]) -> Vec<u8>;
    fn get_hash_bytes_count() -> usize;
}
