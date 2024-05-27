use diff_config::ConfigFile;
use progress_store::ProgressStorageProvider;
use std::{collections::HashMap, ops::BitAnd};
use std::fs::File;
use std::path::Path;
use winbindex::WinbindexEntry;
extern crate tokio;
use crate::{ghidriff::GhidriffDiffingProject, winbindex::{Arch, Winbindex, WinbindexFileData}};

mod diff_config;
mod git;
mod progress_store;
mod winbindex;
mod winbindex_iter;
mod ghidriff;

#[tokio::main]
async fn main() {
    let config_file_path = Path::new("../sample/config.yaml"); //argv[1]

    //let mut config_file = File::open(config_file_path).expect("Could not open file");
    let config_file = diff_config::ConfigFile::open_or_create(config_file_path)
        .expect("Could not open config file");

    println!("{:?}", config_file);
    let store_dir = Path::new(config_file.store_dir.as_str());
    config_file.update_repos().unwrap();
    for (repo_name, repo) in config_file.branches.iter(){
        let instance = repo_name;
        let mut progress_store = ProgressStorageProvider::new(&store_dir);
        let progress = progress_store.get_or_create_branch_store(&repo_name);
        for binary_name in repo.files.iter(){
            let wb = Winbindex::new(Path::new(&config_file.repo_dir).join(repo_name).to_str().unwrap(), &repo.data_dir);
            let file_data = wb.load_file(&binary_name, repo_name).unwrap();
            let json = &file_data.data;
            if progress.none_indexed(binary_name){
                let j = json.clone();
                for (k,v) in json{
                    //println!("{}", v.get_download_url().unwrap().url);
                }
                let amd64 = j.iter().filter_map(|(&ref _k, &ref v)| (v.get_arch()==Arch::Amd64 && v.get_download_url().is_some()).then_some(v.clone())).collect();
                let arm64 = j.iter().filter_map(|(&ref _k, &ref v)| (v.get_arch()==Arch::Arm64&& v.get_download_url().is_some()).then_some(v.clone())).collect();
                let x86 = j.iter().filter_map(|(&ref _k, &ref v)| (v.get_arch()==Arch::X86&& v.get_download_url().is_some()).then_some(v.clone())).collect();
                
                let gd_amd64 = GhidriffDiffingProject::new(Path::new(&config_file.store_dir).to_path_buf(), instance, binary_name,Arch::Amd64);
                let gd_arm64 = GhidriffDiffingProject::new(Path::new(&config_file.store_dir).to_path_buf(), instance, binary_name,Arch::Arm64);
                let gd_x86 = GhidriffDiffingProject::new(Path::new(&config_file.store_dir).to_path_buf(), instance, binary_name,Arch::X86);
                
                let d1 = gd_amd64.run_diff_on_all(&amd64).await.unwrap();
                let d2 = gd_arm64.run_diff_on_all(& arm64).await.unwrap();
                let d3 = gd_x86.run_diff_on_all(& x86).await.unwrap();
                for binary in amd64.iter(){
                    progress.add(binary_name, binary.get_sha256().as_ref());
                }
                for binary in arm64.iter(){
                    progress.add(binary_name, binary.get_sha256().as_ref());
                }
                for binary in x86.iter(){
                    progress.add(binary_name, binary.get_sha256().as_ref());
                }
        
            }
            else{
                
                let next_entry = json.iter()
                .filter(|&(k, _v)| !progress.is_in_index(binary_name, k)).next();
        
        
                if next_entry.is_some(){
                    //1. Find previous for `v`
                    //2. Run diff for `v` and `v-1`
                    //3. update progresstore and flush
                    let hash = next_entry.unwrap().0;
                    let data = next_entry.unwrap().1;
        
        
                    //[1]
                    //Find previous
                    let prev = file_data.find_previous_for_entry(data);
                    
                    //[2]
                    let gd = GhidriffDiffingProject::new(Path::new(&config_file.store_dir).to_path_buf(), instance, binary_name,data.get_arch());
                    let mut bins = Vec::new();
                    if prev.is_none(){
                        continue;
                    }
                    bins.push(prev.unwrap());
                    bins.push(data.clone());
                    let d1 = gd.run_diff_on_all(&bins).await.unwrap();
        
        
                    //[3]
                    progress.add(binary_name, hash);
        
                }
                
            }
        }
        progress_store.flush();

    }



   
    
}
