//! Data structures for handling diffing configuration files
//! See `../sample_config.yaml` for an example
//! 
//! 
use crate::git::{GitError, GitHelper};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
/*
config.yaml
- insider:
  - url: github.com/windbindex/windbinex-insider
  - branch: gh-pages
  - files: ["ntdll.dll"]

*/
#[derive(Debug, Clone)]
pub enum ConfigFileError {
    FileIOError(),
    ImproperlyFormattedConfigFile,
    ConfigFileCreation,
    GitError(GitError),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BranchConfig {
    pub repo_url: String,
    pub branch: String,
    pub data_dir:String,
    pub files: Vec<String>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConfigFile {
    pub branches: HashMap<String, BranchConfig>,
    pub store_dir: String,
    pub repo_dir: String,
}

impl ConfigFile {
    /// Pulls latest updates from Winbindex
    pub fn update_repos(&self) -> Result<(), ConfigFileError> {
        for (k, v) in self.branches.iter() {
            let helper = GitHelper::new(Path::new(&self.repo_dir), &v.branch, &v.repo_url, k);
            helper
                .clone_or_pull()
                .map_err(|err| ConfigFileError::GitError(err))?;
        }
        return Ok(());
    }
    // Opens a config file, or creates one if it does not exist.
    pub(crate) fn open_or_create(path: &Path) -> Result<ConfigFile, ConfigFileError> {
        let mut config_file_result = File::open(path);
        let config: Result<ConfigFile, ConfigFileError>;
        if config_file_result.is_err() {
            config_file_result = File::create(path);
            let mut config_file_result = match config_file_result {
                Ok(file) => file,
                Err(e) => return Err(ConfigFileError::FileIOError()),
            };

            config = Ok(ConfigFile {
                branches: HashMap::new(),
                store_dir: "../sample/store".to_string(),
                repo_dir: "../sample/repos".to_string(),
            });
            let serde_result = serde_yaml::to_writer(
                config_file_result,
                &config.clone().expect("Case shouldn't ever occur"),
            );
            match serde_result {
                Ok(_config_file) => return config,
                Err(_e) => return Err(ConfigFileError::ConfigFileCreation),
            }
        } else {
            let serde_result: Result<ConfigFile, serde_yaml::Error> =
                serde_yaml::from_reader(config_file_result.expect("Case shouldn't ever occur"));
            match serde_result {
                Ok(config_file) => return Ok(config_file),
                Err(_e) => return Err(ConfigFileError::ImproperlyFormattedConfigFile),
            }
        }
    }
}
