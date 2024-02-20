#![cfg_attr(feature = "wasm", no_std)]
#![cfg_attr(feature = "wasm", allow(incomplete_features))]
// #![deny(missing_docs)]
#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]

#[cfg_attr(feature = "wasm", macro_use)]
#[cfg(feature = "wasm")]
extern crate alloc;
#[cfg(feature = "wasm")]
extern crate no_std_compat as std;

pub mod types;

// #[cfg(not(feature = "wasm"))]
mod cache;
// #[cfg(not(feature = "wasm"))]
pub use cache::{Cache, Repo, RepoType};

/// The actual Api to interact with the hub.
#[cfg(any(feature = "online", feature = "tokio", feature = "wasm"))]
pub mod api;

// #[cfg(feature = "wasm")]
// mod wasm_cache;

// Notes:
// - tokens got removed
// - relative redirects got removed - https://github.com/seanmonstar/reqwest/issues/988

/*
#[macro_use]
extern crate alloc;
extern crate no_std_compat as std;

/// The actual Api to interact with the hub.
pub mod api;
pub mod store;

use indexed_db_futures::{web_sys::DomException, IdbDatabase};
use store::*;

use crate::alloc::string::ToString;
use serde::{Deserialize, Serialize};
use std::string::String;
use std::vec::Vec;

#[cfg(feature = "online")]
use rand::{distributions::Alphanumeric, Rng};
// use std::io::Write;
// use std::path::String;

/// The type of repo to interact with
#[derive(Debug, Clone, Copy)]
pub enum RepoType {
    /// This is a model, usually it consists of weight files and some configuration
    /// files
    Model,
    /// This is a dataset, usually contains data within parquet files
    Dataset,
    /// This is a space, usually a demo showcashing a given model or dataset
    Space,
}

/// A local struct used to fetch information from the cache folder.
///
/// Eg. "huggingface/hub".
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(transparent)]
pub struct Cache(pub String);

impl Cache {
    /// Creates a new cache object location
    pub fn new(paths: Vec<String>) -> Self {
        Self(paths.join("/"))
    }

    // /// Returns the location of the token file
    // pub fn token_path(&self) -> String {
    //     let mut paths = self.paths.clone();
    //     // Remove `"hub"`
    //     paths.pop();
    //     paths.push("token".into());
    //     paths.join("/")
    // }

    // /// Returns the token value if it exists in the cache
    // /// Use `huggingface-cli login` to set it up.
    // pub fn token(&self) -> Option<String> {
    //     let token_filename = self.token_path();
    //     if !token_filename.exists() {
    //         log::info!("Token file not found {token_filename:?}");
    //     }
    //     match std::fs::read_to_string(token_filename) {
    //         Ok(token_content) => {
    //             let token_content = token_content.trim();
    //             if token_content.is_empty() {
    //                 None
    //             } else {
    //                 Some(token_content.to_string())
    //             }
    //         }
    //         Err(_) => None,
    //     }
    // }

    /// Creates a new handle [`CacheRepo`] which contains operations
    /// on a particular [`Repo`]
    pub fn repo(&self, repo: Repo) -> CacheRepo {
        CacheRepo::new(self.clone(), repo)
    }

    /// Simple wrapper over
    /// ```
    /// # use hf_hub::{Cache, Repo, RepoType};
    /// # let model_id = "gpt2".to_string();
    /// let cache = Cache::new("/tmp/".into());
    /// let cache = cache.repo(Repo::new(model_id, RepoType::Model));
    /// ```
    pub fn model(&self, model_id: RepoId) -> CacheRepo {
        self.repo(Repo::new(model_id, RepoType::Model))
    }

    /// Simple wrapper over
    /// ```
    /// # use hf_hub::{Cache, Repo, RepoType};
    /// # let model_id = "gpt2".to_string();
    /// let cache = Cache::new("/tmp/".into());
    /// let cache = cache.repo(Repo::new(model_id, RepoType::Dataset));
    /// ```
    pub fn dataset(&self, model_id: RepoId) -> CacheRepo {
        self.repo(Repo::new(model_id, RepoType::Dataset))
    }

    /// Simple wrapper over
    /// ```
    /// # use hf_hub::{Cache, Repo, RepoType};
    /// # let model_id = "gpt2".to_string();
    /// let cache = Cache::new("/tmp/".into());
    /// let cache = cache.repo(Repo::new(model_id, RepoType::Space));
    /// ```
    pub fn space(&self, model_id: RepoId) -> CacheRepo {
        self.repo(Repo::new(model_id, RepoType::Space))
    }

    // pub(crate) fn temp_path(&self) -> String {
    //     let mut path = vec![self.0.clone()];
    //     path.push("tmp".into());
    //     // std::fs::create_dir_all(&path).ok();

    //     // let s: String = rand::thread_rng()
    //     //     .sample_iter(&Alphanumeric)
    //     //     .take(7)
    //     //     .map(char::from)
    //     //     .collect();
    //     // path.push(s);
    //     // path.to_path_buf()
    //     todo!()
    // }
}

/// Shorthand for accessing things within a particular repo
#[derive(Debug)]
pub struct CacheRepo {
    cache: Cache,
    repo: Repo,
}

impl CacheRepo {
    fn new(cache: Cache, repo: Repo) -> Self {
        Self { cache, repo }
    }

    /// Gets a [FileBlobKey] to [DbStore::FileBlob] for the remote `filename`.
    /// Will return `None` if file is not already present in cache.
    ///
    /// ie. "[Cache]/[FolderName]/blobs/[BlobHash]".
    pub async fn get(
        &self,
        db: &IdbDatabase,
        filename: &FilePath,
    ) -> Result<Option<FileBlobKey>, DomException> {
        let commit_path = self.ref_path();
        let commit_hash = self
            .cache
            .store_get::<RevisionHash>(db, DbStore::RefHash, &commit_path.0)
            .await?
            .unwrap();
        let mut pointer_path = vec![self.pointer_path(&commit_hash).0];
        pointer_path.push(filename.0.to_string());
        let file_hash = self
            .cache
            .store_get::<FileBlobKey>(db, DbStore::FileHash, &pointer_path.join("/"))
            .await?;
        Ok(file_hash)
    }

    /// Gets a [HubKey] to [DbStore::Hub].
    ///
    /// Eg. `"huggingface/hub/models--EleutherAI--gpt-neox-20b"`.
    fn path(&self) -> HubKey {
        let mut ref_path = vec![self.cache.0.clone()];
        ref_path.push(self.repo.folder_name().0);
        HubKey(ref_path.join("/"))
    }

    /// Gets a [RefHashKey] to [DbStore::RefHash].
    ///
    /// Eg. `"huggingface/hub/models--EleutherAI--gpt-neox-20b/refs/main"`.
    /// Eg. `"huggingface/hub/models--EleutherAI--gpt-neox-20b/refs/refs/pr/1"`.
    fn ref_path(&self) -> RefHashKey {
        let mut ref_path = vec![self.path().0];
        ref_path.push("refs".into());
        ref_path.push(self.repo.revision.0.to_string());
        RefHashKey(ref_path.join("/"))
    }

    // /// Creates a reference in the cache directory that points branches to the correct
    // /// commits within the blobs.
    // pub fn create_ref(&self, commit_hash: &str) -> Result<(), std::io::Error> {
    //     let ref_path = self.ref_path();
    //     // Needs to be done like this because revision might contain `/` creating subfolders here.
    //     std::fs::create_dir_all(ref_path.parent().unwrap())?;
    //     let mut file = std::fs::OpenOptions::new()
    //         .write(true)
    //         .create(true)
    //         .open(&ref_path)?;
    //     file.write_all(commit_hash.trim().as_bytes())?;
    //     Ok(())
    // }

    /// Gets a [FileBlobKey] to [DbStore::FileBlob] for the file hash `etag`.
    pub(crate) fn blob_path(&self, etag: &BlobHash) -> FileBlobKey {
        let mut blob_path = vec![self.path().0];
        blob_path.push("blobs".into());
        blob_path.push(etag.0.clone());
        FileBlobKey(blob_path.join("/"))
    }

    /// Get a [FilesKey] to [DbStore::Files] for `commit_hash`.
    pub(crate) fn pointer_path(&self, commit_hash: &RevisionHash) -> FilesKey {
        let mut pointer_path = vec![self.path().0];
        pointer_path.push("snapshots".into());
        pointer_path.push(commit_hash.0.clone());
        FilesKey(pointer_path.join("/"))
    }
}

impl Default for Cache {
    fn default() -> Self {
        Self::new(vec!["huggingface".into(), "hub".into()])
    }
}

/// The representation of a repo on the hub.
#[derive(Clone, Debug)]
pub struct Repo {
    repo_id: RepoId,
    repo_type: RepoType,
    revision: RevisionPath,
}

impl Repo {
    /// Repo with the default branch ("main").
    pub fn new(repo_id: RepoId, repo_type: RepoType) -> Self {
        Self::with_revision(repo_id, repo_type, RevisionPath("main".into()))
    }

    /// fully qualified Repo
    pub fn with_revision(repo_id: RepoId, repo_type: RepoType, revision: RevisionPath) -> Self {
        Self {
            repo_id,
            repo_type,
            revision,
        }
    }

    /// Shortcut for [`Repo::new`] with [`RepoType::Model`]
    pub fn model(repo_id: RepoId) -> Self {
        Self::new(repo_id, RepoType::Model)
    }

    /// Shortcut for [`Repo::new`] with [`RepoType::Dataset`]
    pub fn dataset(repo_id: RepoId) -> Self {
        Self::new(repo_id, RepoType::Dataset)
    }

    /// Shortcut for [`Repo::new`] with [`RepoType::Space`]
    pub fn space(repo_id: RepoId) -> Self {
        Self::new(repo_id, RepoType::Space)
    }

    /// The normalized folder nameof the repo within the cache directory
    ///
    /// Returns eg. "models--EleutherAI--gpt-neox-20b"
    pub fn folder_name(&self) -> FolderName {
        let prefix = match self.repo_type {
            RepoType::Model => "models",
            RepoType::Dataset => "datasets",
            RepoType::Space => "spaces",
        };
        FolderName(format!("{prefix}--{}", self.repo_id.0).replace('/', "--"))
    }

    // /// The revision
    // ///
    // /// Eg. `main`, `refs/pr/1`.
    // pub fn revision(&self) -> &str {
    //     &self.revision
    // }

    /// The actual URL part of the repo
    // #[cfg(feature = "online")]
    /// - Models: "[RepoId]"
    /// - Datasets: "datasets/[RepoId]"
    /// - Spaces: "spaces/[RepoId]"
    pub fn url(&self) -> String {
        self.repo_id.url(&self.repo_type)
    }

    // // #[cfg(feature = "online")]
    // /// [RevisionPath] needs to be url escaped before being used in a URL.
    // ///
    // /// Eg. "main", "refs%2Fpr%2F1".
    // pub fn url_revision(&self) -> String {
    //     self.revision.0.replace('/', "%2F")
    // }

    /// Used to compute the repo's url part when accessing the metadata of the repo
    // #[cfg(feature = "online")]
    pub fn api_url(&self) -> String {
        let prefix = match self.repo_type {
            RepoType::Model => "models",
            RepoType::Dataset => "datasets",
            RepoType::Space => "spaces",
        };
        format!(
            "{prefix}/{}/revision/{}",
            self.repo_id.0,
            self.revision.url()
        )
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     #[cfg(not(target_os = "windows"))]
//     fn token_path() {
//         let cache = Cache::default();
//         let token_path = cache.token_path().to_str().unwrap().to_string();
//         if let Ok(hf_home) = std::env::var("HF_HOME") {
//             assert_eq!(token_path, format!("{hf_home}/token"));
//         } else {
//             let n = "huggingface/token".len();
//             assert_eq!(&token_path[token_path.len() - n..], "huggingface/token");
//         }
//     }

//     #[test]
//     #[cfg(target_os = "windows")]
//     fn token_path() {
//         let cache = Cache::default();
//         let token_path = cache.token_path().to_str().unwrap().to_string();
//         if let Ok(hf_home) = std::env::var("HF_HOME") {
//             assert_eq!(token_path, format!("{hf_home}\\token"));
//         } else {
//             let n = "huggingface/token".len();
//             assert_eq!(&token_path[token_path.len() - n..], "huggingface\\token");
//         }
//     }
// }

*/
