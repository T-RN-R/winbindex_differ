//! Manages various Git operations that are needed for the project

use git2::build::RepoBuilder;
use git2::Repository;
use std::path::Path;

#[derive(Debug, Clone)]
pub enum GitError {
    FailedRepoClone,
    RepoFetchFailed,
    CouldNotFindRemote(String),
    CouldNotFindReference,
    MergeAnalysisFailed,
    CouldNotSetTarget,
    SetHeadFailure,
    CheckoutFailure,
}

pub struct GitHelper<'a> {
    repo_dir: &'a Path,
    branch_name: &'a String,
    url: &'a String,
    repo_name: &'a String,
}
impl<'a> GitHelper<'a> {
    pub fn new(
        repository_path: &'a Path,
        branch_name: &'a String,
        repo_url: &'a String,
        repo_name: &'a String,
    ) -> Self {
        GitHelper {
            repo_dir: repository_path,
            branch_name,
            url: repo_url,
            repo_name,
        }
    }
    pub fn pull(&self, repo: &Repository, branch_name: &String) -> Result<(), GitError> {
        let mut remote = repo
            .find_remote("origin")
            .map_err(|_err| GitError::CouldNotFindRemote("origin".to_string()))?;
        remote
            .fetch(&[&branch_name], None, None)
            .map_err(|_err| GitError::RepoFetchFailed)?;

        let fetch_head = repo
            .find_reference("FETCH_HEAD")
            .map_err(|_err| GitError::CouldNotFindReference)?;
        let fetch_commit = repo
            .reference_to_annotated_commit(&fetch_head)
            .map_err(|_err| GitError::CouldNotFindReference)?;
        let analysis = repo
            .merge_analysis(&[&fetch_commit])
            .map_err(|_err| GitError::MergeAnalysisFailed)?;
        if analysis.0.is_up_to_date() {
            Ok(())
        } else if analysis.0.is_fast_forward() {
            let refname = format!("refs/heads/{}", branch_name);
            let mut reference = repo
                .find_reference(&refname)
                .map_err(|_err| GitError::CouldNotFindReference)?;
            reference
                .set_target(fetch_commit.id(), "Fast-Forward")
                .map_err(|_err| GitError::CouldNotSetTarget)?;
            repo.set_head(&refname)
                .map_err(|_err| GitError::SetHeadFailure)?;
            repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))
                .map_err(|_err| GitError::CheckoutFailure)
        } else {
            Err(GitError::RepoFetchFailed)
        }
    }

    ///
    /// Clones a git repository, or pulls it if it already exists.
    ///
    pub fn clone_or_pull(&self) -> Result<Repository, GitError> {
        // First try and create the repo storage location.
        let _ = std::fs::create_dir_all(self.repo_dir);
        let clone_path = &self.repo_dir.join(self.repo_name);
        // Check if it exists first before attempting a full clone;
        let repo = Repository::open(clone_path);

        if repo.is_err() {
            println!("Cloning {}, branch {}", self.url, self.branch_name);
            let repo = RepoBuilder::new()
                .branch(self.branch_name.as_str())
                .clone(self.url, clone_path).expect("");
            Ok(repo)
        } else {
            println!("pulling {}, branch {}", self.url, self.branch_name);

            let r = repo.unwrap();
            self.pull(&r, self.branch_name)
                .map_err(|_err| GitError::FailedRepoClone)?;
            Ok(r)
        }
    }
}
