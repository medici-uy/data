pub trait Hashable {
    fn hashable_data(&self) -> Vec<u8>;
    fn set_hash(&mut self);

    fn hash_data(&self) -> String {
        blake3::hash(&self.hashable_data()).to_string()
    }
}
