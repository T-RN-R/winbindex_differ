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
        return BinaryProgressStore {
            binarys_indexed: HashMap::new(),
        };
    }

    pub fn add(&mut self, filename:&str, hash: &str){
        let mut list = self.binarys_indexed.entry(filename.to_string()).or_insert(Vec::new());
        list.push(hash.to_string());
    }

    pub fn is_in_index(&mut self, filename:&str, hash: &str)->bool{
        let mut list = self.binarys_indexed.entry(filename.to_string()).or_insert(Vec::new());
        list.contains(&hash.to_string())
    }

    pub fn none_indexed(&self, filename:&str) -> bool{
        self.binarys_indexed.get(&filename.to_string()).is_none()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProgressStore {
    store_path: String,
    branches: HashMap<String, BinaryProgressStore>,
}
impl ProgressStore {
    pub fn new(store_path: &str) -> Self {
        return ProgressStore {
            store_path: store_path.to_string(),
            branches: HashMap::new(),
        };
    }
}
#[derive(Debug)]
pub struct ProgressStorageProvider {
    path: String,
    store: ProgressStore,
}
impl ProgressStorageProvider {
    pub fn get_or_create_branch_store(&mut self, name: &str) -> &mut BinaryProgressStore {
        self.store.branches.entry(name.to_string()).or_insert(BinaryProgressStore::new())
    }

    pub fn flush(&self) {
        println!("{:?}", self);
        let file = File::create(self.path.clone()).expect("File does not exist");
        serde_yaml::to_writer(file, &self.store).unwrap();
    }

    pub fn new(path: &Path) -> ProgressStorageProvider {
        let _ = std::fs::create_dir_all(path);
        let progress_file = Path::new(path).join("progress.yaml");
        let mut file = File::open(&progress_file);

        if file.is_err() {
            // Create a new store if not found.
            file = File::create(&progress_file);
            serde_yaml::to_writer(
                file.expect("Could not create file"),
                &ProgressStore::new(path.as_os_str().to_str().unwrap()),
            )
            .expect("Couldn't write yaml");
        }
        file = File::open(&progress_file);
        let fp = file.expect("Could not open or create file");
        let store = serde_yaml::from_reader(fp).expect("invalid YAML");
        return ProgressStorageProvider {
            path: progress_file.as_os_str().to_str().unwrap().to_string(),
            store: store,
        };
    }
}
