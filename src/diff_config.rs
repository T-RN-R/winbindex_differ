//! Data structures for handling diffing configuration files
//! See `../sample_config.yaml` for an example
//! 
use crate::git_utils::{GitError, GitHelper};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

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
        for (k, v) in &self.branches {
            let helper = GitHelper::new(Path::new(&self.repo_dir), &v.branch, &v.repo_url, k);
            helper
                .clone_or_pull()
                .map_err(ConfigFileError::GitError)?;
        }
        Ok(())
    }
    /// Opens a config file, or creates one if it does not exist.
    pub(crate) fn open_or_create(path: &Path) -> Result<Self, ConfigFileError> {
        let mut config_file_result = File::open(path);
        let config: Result<Self, ConfigFileError>;
        if config_file_result.is_err() {
            config_file_result = File::create(path);
            let config_file_result = match config_file_result {
                Ok(file) => file,
                Err(_e) => return Err(ConfigFileError::FileIOError()),
            };

            config = Ok(Self {
                branches: HashMap::new(),
                store_dir: "../sample/store".to_string(),
                repo_dir: "../sample/repos".to_string(),
            });
            let serde_result = serde_yaml::to_writer(
                config_file_result,
                &config.clone().map_err(|_e|ConfigFileError::ConfigFileCreation)?,
            );
            match serde_result {
                Ok(_config_file) => config,
                Err(_e) => Err(ConfigFileError::ConfigFileCreation),
            }
        } else {
            let serde_result: Result<Self, serde_yaml::Error> =
                serde_yaml::from_reader(config_file_result.map_err(|_e|ConfigFileError::ConfigFileCreation)?);
            match serde_result {
                Ok(config_file) => Ok(config_file),
                Err(_e) => Err(ConfigFileError::ImproperlyFormattedConfigFile),
            }
        }
    }
}
