// use super::Cache;
// use crate::alloc::string::ToString;
use crate::RepoType;
// use indexed_db_futures::{
//     idb_object_store::IdbObjectStore,
//     idb_transaction::IdbTransaction,
//     web_sys::{DomException, IdbTransactionMode},
//     IdbDatabase, IdbQuerySource,
// };
use serde::{Deserialize, Serialize};
use std::string::String;
// use std::vec::Vec;
// use wasm_bindgen::JsValue;
// use web_sys::IdbKeyRange;
#[cfg(feature = "wasm")]
use crate::alloc::string::ToString;
use crate::cache::PathType;
use core::str::FromStr;
use std::vec::Vec;

/// Template file for creating a url.
///
/// Eg. "{endpoint}/{repo_id}/resolve/{revision}/{filename}", referring to:  
/// "[Endpoint]/[RepoId]/resolve/[RevisionPath]/[FilePath]".  
///
/// Final url example generated from the template: "https://huggingface.co/EleutherAI/gpt-neox-20b/main/tokenizer.json".
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(transparent)]
pub struct HfUrlTemplate(pub String);

/// Url to a file.
///
/// Eg. "https://huggingface.co/EleutherAI/gpt-neox-20b/main/tokenizer.json".
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(transparent)]
pub struct FileUrl(pub String);

impl HfUrlTemplate {
    /// Get the fully qualified URL of the remote filename
    /// ```
    /// # use hf_hub::api::tokio::Api;
    /// let api = Api::new().unwrap();
    /// let url = api.model("gpt2".to_string()).url("model.safetensors");
    /// assert_eq!(url, "https://huggingface.co/gpt2/resolve/main/model.safetensors");
    /// ```
    pub fn url(
        &self,
        endpoint: &Endpoint,
        repo: &crate::Repo,
        revision: &RevisionPath,
        filename: &FilePath,
    ) -> FileUrl {
        let revision = revision.url();
        let file_url = self
            .0
            .replace("{endpoint}", &endpoint.0)
            .replace("{repo_id}", &repo.url())
            .replace("{revision}", &revision)
            .replace("{filename}", &filename.0);
        FileUrl(file_url)
    }
}

/// Eg. "https://huggingface.co".
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(transparent)]
pub struct Endpoint(pub String);

/// Eg. "EleutherAI/gpt-neox-20b".
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(transparent)]
pub struct RepoId(pub String);

impl RepoId {
    /// The actual URL part of the repo.
    ///
    /// - Models: "[RepoId]"
    /// - Datasets: "datasets/[RepoId]"
    /// - Spaces: "spaces/[RepoId]"
    pub fn url(&self, repo_type: &RepoType) -> String {
        match repo_type {
            RepoType::Model => self.0.clone(),
            RepoType::Dataset => {
                format!("datasets/{}", self.0)
            }
            RepoType::Space => {
                format!("spaces/{}", self.0)
            }
        }
    }
}

/// Eg. "models--EleutherAI--gpt-neox-20b".
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(transparent)]
pub struct FolderName(pub String);

/// Eg. "main", "refs/pr/1".
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(transparent)]
pub struct RevisionPath(pub String);

impl RevisionPath {
    /// [RevisionPath] needs to be url escaped before being used in a URL.
    ///
    /// Eg. "main", "refs%2Fpr%2F1".
    pub fn url(&self) -> String {
        self.0.replace('/', "%2F")
    }
}

/// "{hash}"
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(transparent)]
pub struct RevisionHash(pub String);

/// Eg. "tokenizer.json"
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(transparent)]
pub struct FilePath(pub String);

/// "{hash}"
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(transparent)]
pub struct BlobHash(pub String);

/// A offset part, in bytes, that the chunk refers to.
///
/// This is often used in pairs as [BlobChunkOffset]-[BlobChunkOffset] and refer to the start and end offsets.
///
/// Eg. 0-1: This chunk starts from the beggining (start offset = 0) and contains a single byte of data (end minus the start).
/// If there is a next chunk to this file, it would need to start at 1.
///
/// Eg. 1024-2048: This chunk starts after 1kB (1024 offset) and contains 1kB of data (end minus the start).
/// If there is a next chunk to this file, it would need to start at 2048.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(transparent)]
pub struct BlobChunkOffset(pub usize);

/// Path to a hub, ie. "[Cache]/[FolderName]".
///
/// Eg. `"huggingface/hub/models--EleutherAI--gpt-neox-20b"`.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(transparent)]
pub struct HubPath(pub PathType);

impl FromStr for HubPath {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let split = s.split("/").collect::<Vec<_>>();
        let len = split.len();
        let folder_name = split[len - 1];
        let cache = &split[0..len - 1];
        let cache = crate::Cache::from_str(&cache.join("/"))?;
        let mut hub_path = cache.path;
        hub_path.push(folder_name.to_string());
        Ok(HubPath(hub_path))
    }
}

/// Path to a hub revision, ie. "[Cache]/[FolderName]/refs/[RevisionPath]".
///
/// Eg. `"huggingface/hub/models--EleutherAI--gpt-neox-20b/refs/main"`.  
/// Eg. `"huggingface/hub/models--EleutherAI--gpt-neox-20b/refs/refs/pr/1"`.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(transparent)]
pub struct HubRevPath(pub PathType);

/// Path to a file blob hash, ie. "[Cache]/[FolderName]/blobs/[BlobHash]".
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(transparent)]
pub struct BlobHashPath(pub PathType);

/// Path to the files of a revision, ie. "[Cache]/[FolderName]/snapshots/[RevisionHash]".
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(transparent)]
pub struct FilesPath(pub PathType);

#[cfg(feature = "wasm")]
pub use wasm::*;
#[cfg(feature = "wasm")]
pub mod wasm {
    use super::*;
    use crate::alloc::string::ToString;
    use crate::Cache;
    use indexed_db_futures::{
        // idb_object_store::IdbObjectStore,
        idb_transaction::IdbTransaction,
        web_sys::{DomException, IdbTransactionMode},
        IdbDatabase,
        IdbQuerySource,
    };
    use serde::{Deserialize, Serialize};
    use std::vec::Vec;
    use web_sys::IdbKeyRange;
    // use crate::RepoType;
    // use wasm_bindgen::JsValue;
    // use std::string::String;

    // /// A offset part, in bytes, that the chunk refers to.
    // ///
    // /// This is often used in pairs as [BlobChunkOffset]-[BlobChunkOffset] and refer to the start and end offsets.
    // ///
    // /// Eg. 0-1: This chunk starts from the beggining (start offset = 0) and contains a single byte of data (end minus the start).
    // /// If there is a next chunk to this file, it would need to start at 1.
    // ///
    // /// Eg. 1024-2048: This chunk starts after 1kB (1024 offset) and contains 1kB of data (end minus the start).
    // /// If there is a next chunk to this file, it would need to start at 2048.
    // #[derive(Serialize, Deserialize, Clone, Debug)]
    // #[serde(transparent)]
    // pub struct BlobChunkOffset(pub usize);

    /// Key to [DbStore::Hub], ie. "[Cache]/[FolderName]".
    ///
    /// Eg. `"huggingface/hub/models--EleutherAI--gpt-neox-20b"`.
    pub type HubKey = HubPath;

    impl HubKey {
        /// Splits the components in order.
        pub fn split(&self) -> (Cache, FolderName) {
            // let mut split = self.0.split("/").collect::<Vec<_>>();
            let len: usize = self.0.len();
            let folder_name = &self.0[len - 1];
            let prefix = &self.0[..len - 1];
            (Cache::new(prefix.into()), FolderName(folder_name.into()))
        }
    }

    /// Key to [DbStore::Path], ie. "[Cache]/[FolderName]/refs".
    #[derive(Serialize, Deserialize, Clone, Debug)]
    #[serde(transparent)]
    pub struct PathKey(pub PathType);

    impl PathKey {
        /// Splits the components in order.
        pub fn split(&self) -> (Cache, FolderName) {
            let len: usize = self.0.len();
            let folder_name = &self.0[len - 2];
            let prefix = &self.0[..len - 2];
            (
                Cache::new(prefix.into_iter().map(|s| s.to_string()).collect()),
                FolderName(folder_name.into()),
            )
        }
    }

    /// Key to [DbStore::RefHash], ie. "[Cache]/[FolderName]/refs/[RevisionPath]".
    ///
    /// Eg. `"huggingface/hub/models--EleutherAI--gpt-neox-20b/refs/main"`.  
    /// Eg. `"huggingface/hub/models--EleutherAI--gpt-neox-20b/refs/refs/pr/1"`.
    pub type RefHashKey = HubRevPath;

    /// Key to [DbStore::Files], ie. "[Cache]/[FolderName]/snapshots/[RevisionHash]".
    pub type FilesKey = FilesPath;

    impl FilesKey {
        /// Splits the components in order.
        pub fn split(&self) -> (HubKey, RevisionHash) {
            let len: usize = self.0.len();
            let revision_hash = &self.0[len - 1];
            let prefix = &self.0[..len - 2];
            (HubPath(prefix.into()), RevisionHash(revision_hash.into()))
        }
    }

    /// Key to [DbStore::FileHash], ie. "[Cache]/[FolderName]/snapshots/[RevisionHash]/[FilePath]".
    #[derive(Serialize, Deserialize, Clone, Debug)]
    #[serde(transparent)]
    pub struct FileHashKey(pub PathType);

    /// Key to [DbStore::FileBlob], ie. "[Cache]/[FolderName]/blobs/[BlobHash]".
    pub type FileBlobKey = BlobHashPath;

    impl FileBlobKey {
        /// The `start` and `end` are left-padded with `0`s accordingly to `complete_file_total_size`.
        pub fn chunk(
            &self,
            start: BlobChunkOffset,
            end: BlobChunkOffset,
            complete_file_total_size: BlobChunkOffset,
        ) -> TmpFileBlobKey {
            let start = start.0;
            let end = end.0;
            // gets the width of the complete file total size, so we can left-pad with zeros all offsets.
            // (this is useful for ordering if the keys are strings)
            let width = complete_file_total_size.0.checked_ilog10().unwrap_or(0) + 1;

            let mut parts = self.0.clone();
            parts.push(format!(
                "{start:0width$}-{end:0width$}",
                width = width as usize
            ));
            TmpFileBlobKey(parts)
        }

        /// Splits the components in order.
        pub fn split(&self) -> (HubKey, BlobHash) {
            let len: usize = self.0.len();
            let blob_hash = &self.0[len - 1];
            let prefix = &self.0[..len - 2];
            (HubPath(prefix.into()), BlobHash(blob_hash.into()))
        }
    }

    impl FromStr for BlobHashPath {
        type Err = ();

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            let split = s.split("/").collect::<Vec<_>>();
            let len: usize = split.len();
            let blob_hash = split[len - 1];
            let prefix = &split[..len - 2];

            let hub_path = HubPath::from_str(&prefix.join("/"))?;
            let mut file_blob_key = hub_path.0;
            file_blob_key.push("blobs".into());
            file_blob_key.push(blob_hash.into());

            Ok(BlobHashPath(file_blob_key))
        }
    }

    /// Key to [DbStore::TmpFileBlob], ie. "[Cache]/[FolderName]/blobs/[BlobHash]/[BlobChunkOffset]-[BlobChunkOffset]"
    /// ie. "[FileBlobKey]/[BlobChunkOffset]-[BlobChunkOffset]".
    ///
    /// Note that each stringfied [BlobChunkOffset] should be left-padded with zeros accordingly to it's maximum value.
    #[derive(Serialize, Deserialize, Clone, Debug)]
    #[serde(transparent)]
    pub struct TmpFileBlobKey(pub PathType);

    impl TmpFileBlobKey {
        /// Splits the components in order.
        pub fn split(&self) -> (FileBlobKey, BlobChunkOffset, BlobChunkOffset) {
            // let mut split = self.0.split("/").collect::<Vec<_>>();
            let len: usize = self.0.len();
            let start_end = self.0[len - 1].split("-").collect::<Vec<_>>();
            let prefix = &self.0[..len - 1];
            let start = start_end[0];
            let end = start_end[1];
            (
                BlobHashPath(prefix.into()),
                BlobChunkOffset(start.parse().unwrap()),
                BlobChunkOffset(end.parse().unwrap()),
            )
        }
    }

    impl FromStr for TmpFileBlobKey {
        type Err = ();

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            let mut split = s.split("/").collect::<Vec<_>>();
            let len: usize = split.len();
            let prefix = &split[..len - 1];

            let file_blob_key = FileBlobKey::from_str(&prefix.join("/"))?;
            let mut tmp_file_blob_key = file_blob_key.0;
            tmp_file_blob_key.push(split[len - 1].into());

            Ok(TmpFileBlobKey(tmp_file_blob_key))
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    pub enum DbStore {
        /// All projects within the database. Keys only.
        ///
        /// - Name: "[Cache]".
        /// - Key: "[Cache]/[FolderName]".
        ///   - Eg. "huggingface/hub/models--EleutherAI--gpt-neox-20b".
        /// - Value: "null".
        Hub,

        /// For a project, shows a list of all of it's revisions paths.
        ///
        /// - Name: "[Cache]/path".
        /// - Key: "[Cache]/[FolderName]/refs".
        /// - Value: [[RevisionPath]].
        ///   - Eg. ["refs/pr/1"].
        Path,

        /// For a project revision path, informs it's hash.
        ///
        /// - Name: "[Cache]/ref_hash".
        /// - Key: "[Cache]/[FolderName]/refs/[RevisionPath]".
        /// - Value: "[RevisionHash]".
        RefHash,

        /// For a project revision hash, informs it's files paths.
        ///
        /// - Name: "[Cache]/ref_files".
        /// - Key: "[Cache]/[FolderName]/snapshots/[RevisionHash]".
        /// - Value: "[[FilePath]]".
        ///   - Eg. ["config.json"].
        Files,

        /// For a project revision file path, informs it's hash.
        ///
        /// - Name: "[Cache]/ref_file_hash".
        /// - Key: "[Cache]/[FolderName]/snapshots/[RevisionHash]/[FilePath]".
        /// - Value: "[FolderName]/blobs/[BlobHash]".
        FileHash,

        /// For a project file blob hash, informs it's data.
        ///
        /// - Name: "[Cache]/ref_file_blob".
        /// - Key: "[Cache]/[FolderName]/blobs/[BlobHash]".
        /// - Value: "{blob}".
        FileBlob,

        /// For a project temporary file blob hash, informs a chunk of it's data.
        ///
        /// - Name: "[Cache]/tmp_ref_file_blob".
        /// - Key: "[Cache]/[FolderName]/blobs/[BlobHash]/[BlobChunkOffset]-[BlobChunkOffset]".
        /// - Value: "{blob}".
        TmpFileBlob,
    }

    impl DbStore {
        pub fn name(&self) -> &str {
            match self {
                DbStore::Hub => "",
                DbStore::Path => "path",
                DbStore::RefHash => "ref_hash",
                DbStore::Files => "ref_files",
                DbStore::FileHash => "ref_file_hash",
                DbStore::FileBlob => "ref_file_blob",
                DbStore::TmpFileBlob => "tmp_ref_file_blob",
            }
        }
    }
}
