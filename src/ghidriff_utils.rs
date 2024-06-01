//! Responsible for downloading binaries and harnessing Ghidriff 

use std::{ fs::File, io::copy, path::{Path, PathBuf}, process::Command};

use futures::StreamExt;

use crate::winbindex_utils::{Arch, WinbindexEntry};

extern crate reqwest;
#[derive(Debug)]
pub enum GhidriffError{
    GhidraProjectDirectoryCreation,
    DiffProjectDirectoryCreation,
    BinaryDownloadDirectoryCreation,
    //BinaryNotFoundOnSymbolServer(String),
    WinbindexEntryNoURL,
    Reqwest(reqwest::Error),
    FileWrite(String),
}

pub struct GhidriffDiffingProject {
    store_path: PathBuf,
    winbindex_instance: String,
    binary_name: String,
    arch: Arch,
}



/// Downloads a given `WinbindexEntry` to the provided path. Note that the filename is derived from
/// the `WinbindexEntry`, and is not controllable.
pub async fn download_binary(path: &Path, winbindex_entry:&WinbindexEntry) -> Result<(), GhidriffError>{
    let url = winbindex_entry.get_download_url().ok_or(GhidriffError::WinbindexEntryNoURL)?;
    let response = reqwest::get(url.url).await.map_err(GhidriffError::Reqwest)?;
    let mut dest = {
        let fname = winbindex_entry.get_binary_dlname();
        let fname = path.join(fname);
        if fname.exists(){
            return Ok(());
        }
        File::create(&fname).map_err(|_e|GhidriffError::FileWrite(fname.to_str().unwrap().to_string()))?
    };
    let content =  response.text().await.map_err(GhidriffError::Reqwest)?;
    copy(&mut content.as_bytes(), &mut dest).map_err(|_e|GhidriffError::FileWrite(String::new()))?;

    Ok(())
}

impl GhidriffDiffingProject {
    pub fn new(
        store_path: PathBuf,
        winbindex_instance: &str,
        binary_name: &str,
        arch: Arch,
    ) -> Self {
        Self {
            store_path,
            winbindex_instance: winbindex_instance.to_string(),
            binary_name: binary_name.to_string(),
            arch
        }
    }
    /// Diffs all provided `WinbindexEntry` on a 2-wide sliding window basis. 
    /// ie. entries[0] + entries[1] will be diffed, but so will entries[1] + entries[2]
    pub async fn run_diff_on_all(&self, entries: &Vec<WinbindexEntry>) -> Result<(),GhidriffError> {
        //1. Make temporary directory for binaries
        //2. Download all binaries
        //3. Create a temporary Ghidra project path
        //4. Create a directory for all of the diffs

        //  <store_path>/diffs/<branch>/<filename>/<arch>/<old>-<new>.[md|json]ßßß
        //5. Build ghidriff command with all binary paths
        //6. Run command
        if entries.is_empty(){ 
            println!("Nothing to diff!");
            return Ok(());
        }
        //[1]
        let binary_download_path = self.store_path.join("binaries").join(&self.winbindex_instance).join(&self.binary_name);
        std::fs::create_dir_all(&binary_download_path).map_err(|_e|GhidriffError::BinaryDownloadDirectoryCreation)?;
        
        //[2]
        let fetches = futures::stream::iter(
            entries.iter().map(|entry| {
                async move {
                    let binary_download_path = self.store_path.join("binaries").join(&self.winbindex_instance).join(&self.binary_name);
                    if let Some(_get_download_url) = entry.get_download_url(){
                        match download_binary(&binary_download_path.clone(), &entry.clone()).await {
                            Ok(()) => {
                            }
                            Err(e) => println!("{:?} | ERROR downloading {}", e, entry.get_download_url().unwrap().url),
                        }
                    }
                }
        })
        ).buffer_unordered(8).collect::<Vec<()>>();
            //download_binary(&binary_download_path, winbindex_entry).await?;

        fetches.await;

        //[3]
        let ghidra_projects_path = self.store_path.join("ghidra_projects");
        std::fs::create_dir_all(&ghidra_projects_path).map_err(|_e|GhidriffError::GhidraProjectDirectoryCreation)?;

        //[4]
        let arch_str:String = self.arch.into();
        let diff_folder = &self.store_path.join("diffs").join(&self.winbindex_instance).join(arch_str).join(&self.binary_name);
        std::fs::create_dir_all(diff_folder).map_err(|_e|GhidriffError::DiffProjectDirectoryCreation)?;
        
        let ghidra_runs = futures::stream::iter(
            entries.as_slice().windows(2).map(|chunk| {
                let ghidra_projects_path = ghidra_projects_path.clone();
                let binary_download_path = binary_download_path.clone();
                async move {
                    let old = &chunk[0];
                    let new = &chunk[1];
                    let old_fname = old.get_binary_dlname();
                    let new_fname = new.get_binary_dlname();
                    //[5 + 6]
                    let command = &mut Command::new("ghidriff");
                    let _ghidriff_command = command
                    .arg("-p")
                    .arg(ghidra_projects_path.to_str().unwrap())
                    .arg("-o")
                    .arg(diff_folder.to_str().unwrap())
                    .arg("--force-analysis")
                    .arg("--engine")
                    .arg("VersionTrackingDiff")
                    .arg(binary_download_path.join(old_fname).to_str().unwrap())
                    .arg(binary_download_path.join(new_fname).to_str().unwrap())
                    .status().expect("Could not run Ghidriff");
                }
        })
        ).buffer_unordered(8).collect::<Vec<()>>();
        ghidra_runs.await;
        Ok(())
    }
}
