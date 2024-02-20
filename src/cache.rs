#![deny(missing_docs)]
#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]

#[cfg(any(feature = "online", feature = "tokio"))]
use rand::{distributions::Alphanumeric, Rng};
#[cfg(any(feature = "online", feature = "tokio"))]
use std::io::Write;

use crate::types::{
    BlobHash, BlobHashPath, FilesPath, FolderName, HubPath, HubRevPath, RepoId, RevisionHash,
    RevisionPath,
};

#[cfg(not(any(feature = "online", feature = "tokio", feature = "wasm")))]
pub type PathType = std::path::PathBuf;

#[cfg(any(feature = "online", feature = "tokio"))]
pub type PathType = std::path::PathBuf;

#[cfg(feature = "wasm")]
pub type PathType = Vec<String>;
#[cfg(feature = "wasm")]
mod wasm_imports {
    pub use crate::alloc::{boxed::Box, string::ToString};
    pub use crate::types::{DbStore, FileBlobKey, FilePath};
    pub use indexed_db_futures::{idb_transaction::IdbTransaction, IdbDatabase, IdbQuerySource};
    pub use js_sys::Uint8Array;
    pub use std::{string::String, vec::Vec};
    pub use web_sys::{DomException, IdbKeyRange, IdbTransactionMode};
}
#[cfg(feature = "wasm")]
use wasm_imports::*;

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
#[derive(Clone, Debug)]
pub struct Cache {
    pub(crate) path: PathType,
}

impl Cache {
    /// Creates a new cache object location
    pub fn new(path: PathType) -> Self {
        Self { path }
    }

    /// Creates a new cache object location
    pub fn path(&self) -> &PathType {
        &self.path
    }

    /// Returns the location of the token file
    pub fn token_path(&self) -> PathType {
        let mut path = self.path.clone();
        // Remove `"hub"`
        path.pop();
        path.push("token".to_string());
        path
    }

    #[cfg(any(feature = "online", feature = "tokio"))]
    /// Returns the token value if it exists in the cache
    /// Use `huggingface-cli login` to set it up.
    pub fn token(&self) -> Option<String> {
        let token_filename = self.token_path();
        if !token_filename.exists() {
            log::info!("Token file not found {token_filename:?}");
        }
        match std::fs::read_to_string(token_filename) {
            Ok(token_content) => {
                let token_content = token_content.trim();
                if token_content.is_empty() {
                    None
                } else {
                    Some(token_content.to_string())
                }
            }
            Err(_) => None,
        }
    }

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

    #[cfg(any(feature = "online", feature = "tokio"))]
    pub(crate) fn temp_path(&self) -> PathType {
        let mut path = self.path().clone();
        path.push("tmp");
        std::fs::create_dir_all(&path).ok();

        let s: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(7)
            .map(char::from)
            .collect();
        path.push(s);
        path.to_path_buf()
    }
}

impl std::str::FromStr for Cache {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut path = PathType::default();
        for s in s.split('/') {
            path.push(s.to_string());
        }
        Ok(Self { path })
    }
}

#[cfg(feature = "wasm")]
impl Cache {
    /// Get if existing.
    pub async fn store_get<T: serde::de::DeserializeOwned>(
        &self,
        db: &IdbDatabase,
        store: DbStore,
        key: &str,
    ) -> Result<Option<T>, DomException> {
        let mut store_name = self.path.clone();
        store_name.push(store.name().into());
        let store_name = store_name.join("/");
        let tx: IdbTransaction =
            db.transaction_on_one_with_mode(&store_name, IdbTransactionMode::Readonly)?;
        let store = tx.object_store(&store_name)?;
        let res = store.get_owned(key)?.await?;
        tx.await.into_result()?;
        let res = res.map(|res| serde_wasm_bindgen::from_value(res).unwrap());
        Ok(res)
    }

    /// Get if existing.
    pub async fn store_get_bytes(
        &self,
        db: &IdbDatabase,
        store: DbStore,
        key: &str,
    ) -> Result<Option<Uint8Array>, DomException> {
        let mut store_name = self.path.clone();
        store_name.push(store.name().into());
        let store_name = store_name.join("/");
        let tx: IdbTransaction =
            db.transaction_on_one_with_mode(&store_name, IdbTransactionMode::Readonly)?;
        let store = tx.object_store(&store_name)?;
        let res = store.get_owned(key)?.await?;
        tx.await.into_result()?;
        // TODO: confirm that this doesn't copy the data
        let res = res.map(Uint8Array::from);
        Ok(res)
    }

    /// Get if existing.
    pub async fn store_get_range<T: serde::de::DeserializeOwned>(
        &self,
        db: &IdbDatabase,
        store: DbStore,
        range: &web_sys::IdbKeyRange,
    ) -> Result<Vec<T>, DomException> {
        let mut store_name = self.path.clone();
        store_name.push(store.name().into());
        let store_name = store_name.join("/");
        let tx: IdbTransaction =
            db.transaction_on_one_with_mode(&store_name, IdbTransactionMode::Readonly)?;
        let store = tx.object_store(&store_name)?;
        let res = store.get_all_with_key(range)?.await?;
        tx.await.into_result()?;
        let res = res
            .into_iter()
            .map(|t| serde_wasm_bindgen::from_value(t).unwrap())
            .collect();
        Ok(res)
    }

    /// Add or overwrite.
    pub async fn store_set<T: serde::ser::Serialize>(
        &self,
        db: &IdbDatabase,
        store: DbStore,
        key: &str,
        value: &T,
    ) -> Result<(), DomException> {
        let value = serde_wasm_bindgen::to_value(value).unwrap();
        let mut store_name = self.path.clone();
        store_name.push(store.name().into());
        let store_name = store_name.join("/");
        let tx: IdbTransaction =
            db.transaction_on_one_with_mode(&store_name, IdbTransactionMode::Readwrite)?;
        let store = tx.object_store(&store_name)?;
        store.put_key_val_owned(key, &value)?.await
    }

    /// Add or overwrite.
    pub async fn store_set_bytes(
        &self,
        db: &IdbDatabase,
        store: DbStore,
        key: &str,
        value: &Uint8Array,
    ) -> Result<(), DomException> {
        let mut store_name = self.path.clone();
        store_name.push(store.name().into());
        let store_name = store_name.join("/");
        let tx: IdbTransaction =
            db.transaction_on_one_with_mode(&store_name, IdbTransactionMode::Readwrite)?;
        let store = tx.object_store(&store_name)?;
        store.put_key_val_owned(key, value)?.await
    }

    /// Get list of keys.
    pub async fn store_key_range(
        &self,
        db: &IdbDatabase,
        store: DbStore,
        key_range: IdbKeyRange,
    ) -> Result<Vec<String>, DomException> {
        let mut store_name = self.path.clone();
        store_name.push(store.name().into());
        let store_name = store_name.join("/");
        let tx: IdbTransaction =
            db.transaction_on_one_with_mode(&store_name, IdbTransactionMode::Readonly)?;
        let store = tx.object_store(&store_name)?;
        let cursor = store
            .open_key_cursor_with_range_and_direction_owned(
                key_range,
                web_sys::IdbCursorDirection::Next,
            )?
            .await?;
        let keys = if let Some(cursor) = cursor {
            cursor
                .into_vec(0)
                .await?
                .into_iter()
                .map(|k| k.as_string().unwrap())
                .collect()
        } else {
            vec![]
        };
        tx.await.into_result()?;
        Ok(keys)
    }
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

    #[cfg(any(feature = "online", feature = "tokio"))]
    /// This will get the location of the file within the cache for the remote
    /// `filename`. Will return `None` if file is not already present in cache.
    pub fn get(&self, filename: &str) -> Option<PathType> {
        let commit_path = self.ref_path();
        let commit_hash = RevisionHash(std::fs::read_to_string(commit_path.0).ok()?);
        let mut pointer_path = self.pointer_path(&commit_hash).0;
        pointer_path.push(filename);
        if pointer_path.exists() {
            Some(pointer_path)
        } else {
            None
        }
    }

    #[cfg(feature = "wasm")]
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
        let commit_hash = match self
            .cache
            .store_get::<RevisionHash>(db, DbStore::RefHash, &commit_path.0.join("/"))
            .await?
        {
            Some(rev) => rev,
            // cached file not found, the caller may decide to download it
            None => return Ok(None),
        };
        let mut pointer_path = self.pointer_path(&commit_hash).0;
        pointer_path.push(filename.0.to_string());
        let file_hash = self
            .cache
            .store_get::<FileBlobKey>(db, DbStore::FileHash, &pointer_path.join("/"))
            .await?;
        Ok(file_hash)
    }

    fn path(&self) -> HubPath {
        let mut ref_path = self.cache.path.clone();
        ref_path.push(self.repo.folder_name().0);
        HubPath(ref_path)
    }

    fn ref_path(&self) -> HubRevPath {
        let mut ref_path = self.path().0;
        ref_path.push("refs".to_string());
        ref_path.push(self.repo.revision.0.clone());
        HubRevPath(ref_path)
    }

    #[cfg(any(feature = "online", feature = "tokio"))]
    /// Creates a reference in the cache directory that points branches to the correct
    /// commits within the blobs.
    pub fn create_ref(&self, commit_hash: &RevisionHash) -> Result<(), std::io::Error> {
        let ref_path = self.ref_path();
        // Needs to be done like this because revision might contain `/` creating subfolders here.
        std::fs::create_dir_all(ref_path.0.parent().unwrap())?;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(&ref_path.0)?;
        file.write_all(commit_hash.0.trim().as_bytes())?;
        Ok(())
    }

    pub(crate) fn blob_path(&self, etag: &BlobHash) -> BlobHashPath {
        let mut blob_path = self.path().0;
        blob_path.push("blobs".to_string());
        blob_path.push((&etag.0).to_string());
        BlobHashPath(blob_path)
    }

    pub(crate) fn pointer_path(&self, commit_hash: &RevisionHash) -> FilesPath {
        let mut pointer_path = self.path().0;
        pointer_path.push("snapshots".to_string());
        pointer_path.push((&commit_hash.0).to_string());
        FilesPath(pointer_path)
    }
}

#[cfg(any(feature = "online", feature = "tokio"))]
impl Default for Cache {
    fn default() -> Self {
        let mut path = match std::env::var("HF_HOME") {
            Ok(home) => home.into(),
            Err(_) => {
                let mut cache = dirs::home_dir().expect("Cache directory cannot be found");
                cache.push(".cache");
                cache.push("huggingface");
                cache
            }
        };
        path.push("hub");
        Self::new(path)
    }
}

#[cfg(feature = "wasm")]
impl Default for Cache {
    fn default() -> Self {
        Self::new(vec!["huggingface".into(), "hub".into()])
    }
}

/// The representation of a repo on the hub.
#[derive(Clone, Debug)]
pub struct Repo {
    pub(crate) repo_id: RepoId,
    pub(crate) repo_type: RepoType,
    pub(crate) revision: RevisionPath,
}

impl Repo {
    /// Repo with the default branch ("main").
    pub fn new(repo_id: RepoId, repo_type: RepoType) -> Self {
        Self::with_revision(repo_id, repo_type, RevisionPath("main".to_string()))
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
    pub fn folder_name(&self) -> FolderName {
        let prefix = match self.repo_type {
            RepoType::Model => "models",
            RepoType::Dataset => "datasets",
            RepoType::Space => "spaces",
        };
        FolderName(format!("{prefix}--{}", self.repo_id.0).replace('/', "--"))
    }

    // /// The revision
    // pub fn revision(&self) -> &str {
    //     &self.revision.0
    // }

    /// The actual URL part of the repo
    pub fn url(&self) -> String {
        self.repo_id.url(&self.repo_type)
    }

    // /// Revision needs to be url escaped before being used in a URL
    // pub fn url_revision(&self) -> String {
    //     self.revision.0.replace('/', "%2F")
    // }

    /// Used to compute the repo's url part when accessing the metadata of the repo
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

#[cfg(test)]
#[cfg(any(feature = "online", feature = "tokio", feature = "wasm"))]
mod tests {
    use super::*;

    #[test]
    #[cfg(all(not(target_os = "windows"), not(feature = "wasm")))]
    fn token_path() {
        let cache = Cache::default();
        let token_path = cache.token_path().to_str().unwrap().to_string();
        if let Ok(hf_home) = std::env::var("HF_HOME") {
            assert_eq!(token_path, format!("{hf_home}/token"));
        } else {
            let n = "huggingface/token".len();
            assert_eq!(&token_path[token_path.len() - n..], "huggingface/token");
        }
    }

    #[test]
    #[cfg(all(target_os = "windows", not(feature = "wasm")))]
    fn token_path() {
        let cache = Cache::default();
        let token_path = cache.token_path().to_str().unwrap().to_string();
        if let Ok(hf_home) = std::env::var("HF_HOME") {
            assert_eq!(token_path, format!("{hf_home}\\token"));
        } else {
            let n = "huggingface/token".len();
            assert_eq!(&token_path[token_path.len() - n..], "huggingface\\token");
        }
    }
}
