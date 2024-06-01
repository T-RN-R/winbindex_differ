//! Diffs Windows binarys based upon Winbindex metadata. Uses Ghidriff to power the diffs.
//! Intended to be run in CI/CD to produce continuous diffs.

#![warn(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::cargo,
)]

use progress::StorageProvider;
use std::path::Path;
extern crate tokio;
use crate::{ghidriff_utils::GhidriffDiffingProject, winbindex_utils::{Arch, Winbindex}};

mod diff_config;
mod git_utils;
mod progress;
mod winbindex_utils;
mod ghidriff_utils;

#[tokio::main]
async fn main() {
    let config_file_path = Path::new("../sample/config.yaml"); //argv[1]

    let config_file = diff_config::ConfigFile::open_or_create(config_file_path)
        .expect("Could not open config file");

    let store_dir = Path::new(config_file.store_dir.as_str());
    config_file.update_repos().unwrap();

    // iterate through all provided Winbindex Git repositorys, this will be arm64, x64 and insider.
    for (repo_name, repo) in &config_file.branches{
        let instance = repo_name;
        let mut progress_store = StorageProvider::new(store_dir);
        let progress = progress_store.get_or_create_branch_store(repo_name);
        // iterate through all of the binarys for which  we wish to generate diffs
        for binary_name in &repo.files{
            let wb = Winbindex::new(Path::new(&config_file.repo_dir)
                .join(repo_name).to_str().unwrap(), &repo.data_dir);
            let file_data = wb.load_file(binary_name, repo_name).unwrap();
            let json = &file_data.data;

            // If this binary has not been seen before as per the progress storage file, diff all
            // versions of it
            if progress.none_indexed(binary_name){
                let j = json.clone();

                let amd64 = j.iter().filter_map(|( _k,  v)| (v.get_arch()==Arch::Amd64 && v.get_download_url().is_some()).then_some(v.clone())).collect();
                let arm64 = j.iter().filter_map(|( _k, v)| (v.get_arch()==Arch::Arm64&& v.get_download_url().is_some()).then_some(v.clone())).collect();
                let x86 = j.iter().filter_map(|( _k, v)| (v.get_arch()==Arch::X86&& v.get_download_url().is_some()).then_some(v.clone())).collect();
                
                let gd_amd64 = GhidriffDiffingProject::new(Path::new(&config_file.store_dir).to_path_buf(), instance, binary_name,Arch::Amd64);
                let gd_arm64 = GhidriffDiffingProject::new(Path::new(&config_file.store_dir).to_path_buf(), instance, binary_name,Arch::Arm64);
                let gd_x86 = GhidriffDiffingProject::new(Path::new(&config_file.store_dir).to_path_buf(), instance, binary_name,Arch::X86);
                
                gd_amd64.run_diff_on_all(&amd64).await.unwrap();
                gd_arm64.run_diff_on_all(& arm64).await.unwrap();
                gd_x86.run_diff_on_all(& x86).await.unwrap();
                for binary in &amd64{
                    progress.add(binary_name, binary.get_sha256().as_ref());
                }
                for binary in &arm64{
                    progress.add(binary_name, binary.get_sha256().as_ref());
                }
                for binary in &x86{
                    progress.add(binary_name, binary.get_sha256().as_ref());
                }
        
            }
            else{
                
                let next_entry = json.iter()
                    .find(|&(k, _v)| !progress.is_in_index(binary_name, k));
        
        
                if next_entry.is_some(){
                    //1. Find previous for `v`
                    //2. Run diff for `v` and `v-1`
                    //3. update progresstore and flush
                    let hash = next_entry.unwrap().0;
                    let data = next_entry.unwrap().1;
        
        
                    //[1]
                    //Find previous
                    let prev = file_data.find_previous_for_entry(data);
                    
                    //[2] run diff
                    let gd = GhidriffDiffingProject::new(Path::new(&config_file.store_dir).to_path_buf(), instance, binary_name,data.get_arch());
                    let mut bins = Vec::new();
                    if prev.is_none(){
                        continue;
                    }
                    bins.push(prev.unwrap());
                    bins.push(data.clone());
                    gd.run_diff_on_all(&bins).await.unwrap();
        
        
                    //[3] add to progress store
                    progress.add(binary_name, hash);
        
                }
                
            }
        }
        progress_store.flush();

    }



   
    
}
