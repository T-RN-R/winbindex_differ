//! Manages progress across multiple runs of the program. This is helpful for CI/CD scenarios.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

#[derive(Serialize, Deserialize, Debug)]
pub struct BinaryProgressStore {
    binarys_indexed: HashMap<String, Vec<String>>, // binary_name : [hash1, hash2]
}
impl BinaryProgressStore {
    pub fn new() -> Self {
        Self {
            binarys_indexed: HashMap::new(),
        }
    }
    /// Add an entry to the store.
    pub fn add(&mut self, filename:&str, hash: &str){
        let list = self.binarys_indexed.entry(filename.to_string()).or_default();
        list.push(hash.to_string());
    }
    /// Checks if a binary+hash combo exists in the store.
    pub fn is_in_index(&mut self, filename:&str, hash: &str)->bool{
        let list = self.binarys_indexed.entry(filename.to_string()).or_default();
        list.contains(&hash.to_string())
    }
    /// Checks if there is no entry for a given binary.
    pub fn none_indexed(&self, filename:&str) -> bool{
        self.binarys_indexed.get(&filename.to_string()).is_none()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Store {
    store_path: String,
    branches: HashMap<String, BinaryProgressStore>,
}
impl Store {
    pub fn new(store_path: &str) -> Self {
        Self {
            store_path: store_path.to_string(),
            branches: HashMap::new(),
        }
    }
}
#[derive(Debug)]
pub struct StorageProvider {
    path: String,
    store: Store,
}
impl StorageProvider {
    /// Gets the store for a given branch.
    pub fn get_or_create_branch_store(&mut self, name: &str) -> &mut BinaryProgressStore {
        self.store.branches.entry(name.to_string()).or_insert_with(BinaryProgressStore::new)
    }
    /// Flush the store to disk.
    pub fn flush(&self) {
        let file = File::create(self.path.clone()).expect("File does not exist");
        serde_yaml::to_writer(file, &self.store).unwrap();
    }
    /// Create a new store.
    pub fn new(path: &Path) -> Self {
        let _ = std::fs::create_dir_all(path);
        let progress_file = Path::new(path).join("progress.yaml");
        let mut file = File::open(&progress_file);

        if file.is_err() {
            // Create a new store if not found.
            file = File::create(&progress_file);
            serde_yaml::to_writer(
                file.expect("Could not create file"),
                &Store::new(path.as_os_str().to_str().unwrap()),
            )
            .expect("Couldn't write yaml");
        }
        file = File::open(&progress_file);
        let fp = file.expect("Could not open or create file");
        let store = serde_yaml::from_reader(fp).expect("invalid YAML");
        Self {
            path: progress_file.as_os_str().to_str().unwrap().to_string(),
            store,
        }
    }
}
