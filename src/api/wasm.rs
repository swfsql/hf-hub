use super::RepoInfo;
use crate::{Cache, Repo, RepoType};
use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};
use core::str::FromStr;
use indexed_db_futures::IdbDatabase;
use js_sys::Uint8Array;
// use indicatif::{ProgressBar, ProgressStyle};
// use rand::Rng;
use reqwest::{
    header::{
        HeaderMap, HeaderName, HeaderValue, InvalidHeaderValue, ToStrError, AUTHORIZATION,
        CONTENT_RANGE, LOCATION, RANGE, USER_AGENT,
    },
    // redirect::Policy,
    Client,
    Error as ReqwestError,
    RequestBuilder,
};
use std::{collections::HashSet, num::ParseIntError};
use wasm_bindgen::JsValue;
// use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
// use tokio::io::{AsyncSeekExt, AsyncWriteExt, SeekFrom};
// use tokio::sync::{AcquireError, Semaphore, TryAcquireError};
use crate::types::{wasm::*, *};

/// Current version (used in user-agent)
const VERSION: &str = env!("CARGO_PKG_VERSION");
/// Current name (used in user-agent)
const NAME: &str = env!("CARGO_PKG_NAME");

#[derive(Debug, snafu::Snafu)]
/// All errors the API can throw
pub enum ApiError {
    // /// Api expects certain header to be present in the results to derive some information
    // #[snafu(display("Header {header_name} is missing"))]
    // MissingHeader { header_name: HeaderName },

    // /// The header exists, but the value is not conform to what the Api expects.
    // #[snafu(display("Header {header_name} is invalid"))]
    // InvalidHeader { header_name: HeaderName },

    // /// The value cannot be used as a header during request header construction
    // #[snafu(display("Invalid header value {header_value}"))]
    // InvalidHeaderValue {
    //     // #[from]
    //     header_value: InvalidHeaderValue,
    // },
    /// The header value is not valid utf-8
    #[snafu(display("header value is not a string"))]
    ToStr {
        // #[from]
        source: ToStrError,
    },

    /// Error in the request
    #[snafu(display("request error: {reqwest_error}"))]
    RequestError {
        // #[from]
        reqwest_error: ReqwestError,
    },

    /// Error parsing some range value
    #[snafu(display("Cannot parse int"))]
    ParseIntError {
        // #[from]
        source: ParseIntError,
    },
    // /// I/O Error
    // #[snafu(display("I/O error {0}"))]
    // IoError {
    //     #[from]
    //     io_error: std::io::Error,
    // },
    // /// We tried to download chunk too many times
    // #[snafu(display("Too many retries: {api_error}"))]
    // TooManyRetries { api_error: Box<ApiError> },
    // /// Semaphore cannot be acquired
    // #[snafu(display("Try acquire: {try_acquire_error}"))]
    // TryAcquireError {
    //     #[from]
    //     try_acquire_error: TryAcquireError,
    // },
    // /// Semaphore cannot be acquired
    // #[snafu(display("Acquire: {acquire_error}"))]
    // AcquireError {
    //     #[from]
    //     acquire_error: AcquireError,
    // },
    // /// Semaphore cannot be acquired
    // #[error("Invalid Response: {0:?}")]
    // InvalidResponse(Response),
}

#[derive(Clone, Debug, PartialEq)]
pub enum UrlTemplate {
    Hf(HfUrlTemplate),
    Custom(String),
}

impl Default for UrlTemplate {
    /// [HfUrlTemplate] as "{endpoint}/{repo_id}/resolve/{revision}/{filename}".
    fn default() -> Self {
        Self::Hf(HfUrlTemplate::default())
    }
}

impl UrlTemplate {
    pub fn as_str(&self) -> &str {
        match self {
            UrlTemplate::Hf(template) => &template.0,
            UrlTemplate::Custom(template) => template,
        }
    }
    pub fn url(
        &self,
        endpoint: &Endpoint,
        repo: &crate::Repo,
        revision: &RevisionPath,
        filename: &FilePath,
    ) -> FileUrl {
        match self {
            UrlTemplate::Hf(template) => template.url(endpoint, repo, revision, filename),
            UrlTemplate::Custom(_template) => todo!(),
        }
    }
}

/// Helper to create [`Api`] with all the options.
#[derive(Debug)]
pub struct ApiBuilder {
    endpoint: Endpoint,
    cache: Cache,
    url_template: UrlTemplate,
    // token: Option<String>,
    // TODO: check if required
    max_files: usize,
    /// Size, in bytes, for each download chunk.
    chunk_size: usize,
    parallel_failures: usize,
    max_retries: usize,
    progress: bool,
}

impl Default for ApiBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ApiBuilder {
    /// Default api builder
    /// ```
    /// use hf_hub::api::tokio::ApiBuilder;
    /// let api = ApiBuilder::new().build().unwrap();
    /// ```
    pub fn new() -> Self {
        let cache = Cache::default();
        Self::with(cache, Endpoint::default(), UrlTemplate::default())
    }

    /// From a given cache
    /// ```
    /// use hf_hub::{api::tokio::ApiBuilder, Cache};
    /// let path = std::path::PathBuf::from("/tmp");
    /// let cache = Cache::new(path);
    /// let api = ApiBuilder::from_cache(cache).build().unwrap();
    /// ```
    pub fn with(cache: Cache, endpoint: Endpoint, url_template: UrlTemplate) -> Self {
        // let token = cache.token();

        let progress = true;

        Self {
            endpoint,
            url_template,
            cache,
            // token,
            max_files: 1,
            // 10MB
            chunk_size: 10 * 1024 * 1024,
            parallel_failures: 0,
            max_retries: 0,
            progress,
        }
    }

    /// Wether to show a progressbar
    pub fn with_progress(mut self, progress: bool) -> Self {
        self.progress = progress;
        self
    }

    /// Changes the location of the cache directory.
    pub fn with_cache_dir(mut self, cache_dir: Cache) -> Self {
        self.cache = cache_dir;
        self
    }

    // /// Sets the token to be used in the API
    // pub fn with_token(mut self, token: Option<String>) -> Self {
    //     self.token = token;
    //     self
    // }

    fn build_headers(&self) -> Result<HeaderMap, ApiError> {
        let mut headers = HeaderMap::new();
        let user_agent = format!("unkown/None; {NAME}/{VERSION}; rust/unknown");
        headers.insert(USER_AGENT, HeaderValue::from_str(&user_agent).unwrap());
        // if let Some(token) = &self.token {
        //     headers.insert(
        //         AUTHORIZATION,
        //         HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
        //     );
        // }
        Ok(headers)
    }

    /// Consumes the builder and builds the final [`Api`]
    pub async fn build(self) -> Result<Api, ApiError> {
        log::info!("Building Api");
        let headers = self.build_headers().unwrap();
        let client = Client::builder()
            .default_headers(headers.clone())
            .build()
            .unwrap();

        // // Policy: only follow relative redirects
        // // See: https://github.com/huggingface/huggingface_hub/blob/9c6af39cdce45b570f0b7f8fad2b311c96019804/src/huggingface_hub/file_download.py#L411
        // let relative_redirect_policy = Policy::custom(|attempt| {
        //     // Follow redirects up to a maximum of 10.
        //     if attempt.previous().len() > 10 {
        //         return attempt.error("too many redirects");
        //     }

        //     if let Some(last) = attempt.previous().last() {
        //         // If the url is not relative
        //         if last.make_relative(attempt.url()).is_none() {
        //             return attempt.stop();
        //         }
        //     }

        //     // Follow redirect
        //     attempt.follow()
        // });

        // let client = Client::builder()
        //     // .redirect(relative_redirect_policy)
        //     .default_headers(headers)
        //     .build()
        //     .unwrap();

        let db_client = {
            use indexed_db_futures::prelude::*;
            const DB_NAME: &str = "HUGGINGFACE_DB";
            const DB_VERSION: u32 = 2;
            let path = self.cache.path.clone();
            let mut db_req: OpenDbRequest = IdbDatabase::open_u32(DB_NAME, DB_VERSION).unwrap();

            // db_req.set_on_upgrade_needed(Some(
            db_req.set_on_upgrade_needed(Some(
                move |evt: &IdbVersionChangeEvent| -> Result<(), JsValue> {
                    let required_stores: HashSet<String> = [
                        DbStore::Hub,
                        DbStore::Path,
                        DbStore::RefHash,
                        DbStore::Files,
                        DbStore::FileHash,
                        DbStore::FileBlob,
                        DbStore::TmpFileBlob,
                    ]
                    .into_iter()
                    .map(|s| {
                        let mut store_name = path.clone();
                        store_name.push(s.name().into());
                        let name = store_name.join("/");
                        // log::info!("Setting up store {name}");
                        #[allow(clippy::let_and_return)]
                        name
                    })
                    .collect();
                    let current_stores: HashSet<String> = evt.db().object_store_names().collect();
                    for store in required_stores.difference(&current_stores) {
                        log::info!("Creating the IndexDB object store: {store}");
                        evt.db().create_object_store(store)?;
                    }
                    Ok(())
                },
            ));

            let db: IdbDatabase = db_req.await.unwrap();
            log::info!("Finished Building Api");
            db
        };

        Ok(Api {
            endpoint: self.endpoint,
            url_template: self.url_template,
            cache: self.cache,
            client,
            db_client: Arc::new(db_client),
            // relative_redirect_client,
            // max_files: self.max_files,
            chunk_size: self.chunk_size,
            // parallel_failures: self.parallel_failures,
            // max_retries: self.max_retries,
            // progress: self.progress,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Metadata {
    pub commit_hash: RevisionHash,
    pub etag: BlobHash,
    pub size: usize,
}

/// The actual Api used to interact with the hub.
/// You can inspect repos with [`Api::info`]
/// or download files with [`Api::download`]
#[derive(Debug, Clone)]
pub struct Api {
    endpoint: Endpoint,
    url_template: UrlTemplate,
    cache: Cache,
    client: Client,
    db_client: Arc<IdbDatabase>,
    // relative_redirect_client: Client,
    // max_files: usize,
    chunk_size: usize,
    // parallel_failures: usize,
    // max_retries: usize,
    // progress: bool,
}

// fn make_relative(src: &Path, dst: &Path) -> PathBuf {
//     let path = src;
//     let base = dst;

//     assert_eq!(
//         path.is_absolute(),
//         base.is_absolute(),
//         "This function is made to look at absolute paths only"
//     );
//     let mut ita = path.components();
//     let mut itb = base.components();

//     loop {
//         match (ita.next(), itb.next()) {
//             (Some(a), Some(b)) if a == b => (),
//             (some_a, _) => {
//                 // Ignoring b, because 1 component is the filename
//                 // for which we don't need to go back up for relative
//                 // filename to work.
//                 let mut new_path = PathBuf::new();
//                 for _ in itb {
//                     new_path.push(Component::ParentDir);
//                 }
//                 if let Some(a) = some_a {
//                     new_path.push(a);
//                     for comp in ita {
//                         new_path.push(comp);
//                     }
//                 }
//                 return new_path;
//             }
//         }
//     }
// }

// fn symlink_or_rename(src: &Path, dst: &Path) -> Result<(), std::io::Error> {
//     if dst.exists() {
//         return Ok(());
//     }

//     let rel_src = make_relative(src, dst);
//     #[cfg(target_os = "windows")]
//     {
//         if std::os::windows::fs::symlink_file(rel_src, dst).is_err() {
//             std::fs::rename(src, dst).unwrap();
//         }
//     }

//     #[cfg(target_family = "unix")]
//     std::os::unix::fs::symlink(rel_src, dst).unwrap();

//     Ok(())
// }

// fn jitter() -> usize {
//     rand::thread_rng().gen_range(0..=500)
// }

// fn exponential_backoff(base_wait_time: usize, n: usize, max: usize) -> usize {
//     (base_wait_time + n.pow(2) + jitter()).min(max)
// }

impl Api {
    /// Creates a default Api, for Api options See [`ApiBuilder`]
    pub async fn new() -> Result<Self, ApiError> {
        ApiBuilder::new().build().await
    }

    /// Get the underlying api client
    /// Allows for lower level access
    pub fn client(&self) -> &Client {
        &self.client
    }

    // TODO: check why there is a CORS error when there is a redirect to S3 on Firefox
    // (works on chromium)
    pub async fn metadata(&self, url: &FileUrl) -> Result<Metadata, ApiError> {
        log::info!("downloading metadata: {}", url.0);

        let response = self
            .client
            .get(&url.0)
            .header(RANGE, "bytes=0-0")
            .send()
            .await
            .unwrap();

        let headers = response.headers();
        let header_commit = HeaderName::from_static("x-repo-commit");
        let header_linked_etag = HeaderName::from_static("x-linked-etag");
        let header_etag = HeaderName::from_static("etag");

        let etag = match headers.get(&header_linked_etag) {
            Some(etag) => etag,
            None => headers
                .get(&header_etag)
                // .ok_or(ApiError::MissingHeader(header_etag))
                .unwrap(),
        };
        // Cleaning extra quotes
        let etag = etag.to_str().unwrap().to_string().replace('"', "");
        let commit_hash = headers
            .get(&header_commit)
            // .ok_or(ApiError::MissingHeader(header_commit))
            .map(|hc| hc.to_str().unwrap().to_string())
            // TODO: BUG: any unknown commit_hash would be considered an "empty string".
            // this should be fixed somehow
            //
            // This is problematic because there is no way to get the headers that
            // contain the information from using the fetch API.
            //
            // I read something about webworkers possibly helping in this?
            // (unsure, but if they can do atomic redirects, this could work)
            .unwrap_or_default();

        let headers = response.headers();
        let content_range = headers
            .get(CONTENT_RANGE)
            // .ok_or(ApiError::MissingHeader(CONTENT_RANGE))
            .unwrap()
            .to_str()
            .unwrap();

        let size = content_range
            .split('/')
            .last()
            // .ok_or(ApiError::InvalidHeader(CONTENT_RANGE))
            .unwrap()
            .parse()
            .unwrap();
        Ok(Metadata {
            commit_hash: RevisionHash(commit_hash),
            etag: BlobHash(etag),
            size,
        })
    }

    /// Creates a new handle [`ApiRepo`] which contains operations
    /// on a particular [`Repo`]
    pub fn repo(&self, repo: Repo) -> ApiRepo {
        ApiRepo::new(self.clone(), repo)
    }

    /// Simple wrapper over
    /// ```
    /// # use hf_hub::{api::tokio::Api, Repo, RepoType};
    /// # let model_id = "gpt2".to_string();
    /// let api = Api::new().unwrap();
    /// let api = api.repo(Repo::new(model_id, RepoType::Model));
    /// ```
    pub fn model(&self, model_id: RepoId) -> ApiRepo {
        self.clone().repo(Repo::new(model_id, RepoType::Model))
    }

    /// Simple wrapper over
    /// ```
    /// # use hf_hub::{api::tokio::Api, Repo, RepoType};
    /// # let model_id = "gpt2".to_string();
    /// let api = Api::new().unwrap();
    /// let api = api.repo(Repo::new(model_id, RepoType::Dataset));
    /// ```
    pub fn dataset(&self, model_id: RepoId) -> ApiRepo {
        self.clone().repo(Repo::new(model_id, RepoType::Dataset))
    }

    /// Simple wrapper over
    /// ```
    /// # use hf_hub::{api::tokio::Api, Repo, RepoType};
    /// # let model_id = "gpt2".to_string();
    /// let api = Api::new().unwrap();
    /// let api = api.repo(Repo::new(model_id, RepoType::Space));
    /// ```
    pub fn space(&self, model_id: RepoId) -> ApiRepo {
        self.clone().repo(Repo::new(model_id, RepoType::Space))
    }

    pub async fn load<T: serde::de::DeserializeOwned>(&self, blob_key: &TmpFileBlobKey) -> T {
        self.cache
            .store_get::<T>(&self.db_client, DbStore::TmpFileBlob, &blob_key.0.join("/"))
            .await
            .unwrap()
            .unwrap()
    }
    pub async fn load_range<T: serde::de::DeserializeOwned>(
        &self,
        range: &web_sys::IdbKeyRange,
    ) -> Vec<T> {
        self.cache
            .store_get_range::<T>(&self.db_client, DbStore::TmpFileBlob, range)
            .await
            .unwrap()
    }

    pub async fn load_bytes(&self, blob_keys: &[TmpFileBlobKey]) -> Vec<u8> {
        let (_, _last_start, last_end) = blob_keys.last().unwrap().split();
        let mut res = Vec::with_capacity(last_end.0);
        for key in blob_keys {
            let bytes = self
                .cache
                .store_get_bytes(&self.db_client, DbStore::TmpFileBlob, &key.0.join("/"))
                .await
                .unwrap()
                .unwrap();
            // let bytes = self.
            // let bytes = self.load::<Vec<u8>>(key).await;
            res.extend(bytes.to_vec());
        }
        res
    }

    pub async fn delete_bytes(&self, blob_keys: &[TmpFileBlobKey]) {
        for key in blob_keys {
            let () = self
                .cache
                .store_delete(&self.db_client, DbStore::TmpFileBlob, &key.0.join("/"))
                .await
                .unwrap();
        }
    }

    // note: this use the range, but there are no performance gains
    // pub async fn load_bytes(&self, blob_keys: &[TmpFileBlobKey]) -> Vec<u8> {
    //     let first = blob_keys.first().unwrap();
    //     let first = JsValue::from_str(&first.0.join("/"));
    //     let last = blob_keys.last().unwrap();
    //     let (_, _last_start, last_end) = last.split();
    //     let last = JsValue::from_str(&last.0.join("/"));
    //     // note: assumes theres no extra data between the first and start (no repeated chunks)
    //     let range = web_sys::IdbKeyRange::bound(&first, &last).unwrap();

    //     let mut res = Vec::with_capacity(last_end.0);
    //     let bytes_vec = self.load_range::<Vec<u8>>(&range).await;
    //     for bytes in bytes_vec {
    //         res.extend(bytes);
    //     }

    //     // for key in blob_keys {
    //     //     let bytes = self.load::<Vec<u8>>(key).await;
    //     //     res.extend(bytes);
    //     // }
    //     res
    // }
}

/// Shorthand for accessing things within a particular repo
#[derive(Debug)]
pub struct ApiRepo {
    api: Api,
    repo: Repo,
}

impl ApiRepo {
    fn new(api: Api, repo: Repo) -> Self {
        Self { api, repo }
    }
}

impl ApiRepo {
    /// Get the fully qualified URL of the remote filename
    /// ```
    /// # use hf_hub::api::tokio::Api;
    /// let api = Api::new().unwrap();
    /// let url = api.model("gpt2".to_string()).url("model.safetensors");
    /// assert_eq!(url, "https://huggingface.co/gpt2/resolve/main/model.safetensors");
    /// ```
    pub fn url(&self, filename: &FilePath) -> FileUrl {
        self.api.url_template.url(
            &self.api.endpoint,
            &self.repo,
            &self.repo.revision,
            filename,
        )
    }

    // TODO: rename function (get/download/etc)
    async fn download_tempfiles(
        &self,
        chunks: TmpFileBlobKeyList,
        url: &FileUrl,
    ) -> Result<Vec<TmpFileBlobKey>, ApiError> {
        log::info!("Checking/downloading file as chunks: {}", &url.0);
        let mut ok_chunks = vec![];
        for chunk in chunks {
            match chunk {
                Ok(chunk) => {
                    ok_chunks.push(chunk);
                }
                Err(chunk) => {
                    let () = self.download_tempfile(url, &chunk).await?;
                    ok_chunks.push(chunk);
                }
            }
        }
        // log::info!(
        //     "Checked/downloaded file as chunks: {} ({} chunks)",
        //     &url.0,
        //     ok_chunks.len()
        // );
        Ok(ok_chunks)
    }

    pub async fn download_tempfile(
        &self,
        url: &FileUrl,
        chunk_file: &TmpFileBlobKey,
    ) -> Result<(), ApiError> {
        let (_prefix, byte_start, byte_end) = chunk_file.split();
        let size_bytes = byte_end.0 - byte_start.0;
        log::info!("Downloading chunk: {}", &chunk_file.0.join("/"));

        // TODO: explain why this - 1 is required, also on documentations
        let chunk_data: bytes::Bytes =
            Self::download_chunk(&self.api.client, url, byte_start.0, byte_end.0 - 1)
                .await
                .unwrap();
        // TODO: allow for the returned data to be larger than the expected size, and react accordingly? (maybe not as this is assumed to never happen)
        // TODO: discard extra data in case it's present?
        assert_eq!(size_bytes, chunk_data.len());
        assert!(size_bytes <= u32::MAX as usize);

        // TODO: consider the unsafe fn Uint8Array::view() to avoid a copy
        let chunk_data_arr = Uint8Array::new_with_length(size_bytes as u32);
        chunk_data_arr.copy_from(&chunk_data);
        drop(chunk_data);

        let () = self
            .api
            .cache
            .store_set_bytes(
                &self.api.db_client,
                DbStore::TmpFileBlob,
                &chunk_file.0.join("/"),
                &chunk_data_arr,
            )
            .await
            .unwrap();
        // log::info!(
        //     "Saved chunk to storage: {} ({} bytes)",
        //     &chunk_file.0.join("/"),
        //     size_bytes
        // );
        Ok(())
    }

    // TODO: rename function (get/download/etc)
    // TODO: have a max retries
    async fn download_chunk(
        client: &reqwest::Client,
        url: &FileUrl,
        start: usize,
        end: usize,
    ) -> Result<bytes::Bytes, ApiError> {
        let range = format!("bytes={start}-{end}");
        let response = client
            .get(&url.0)
            .header(RANGE, range)
            // .fetch_mode_no_cors()
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap();
        Ok(response.bytes().await.unwrap())
    }

    // TODO: rename function (get/download/etc)
    //
    /// This will attempt the fetch the file locally first, then [`Api.download`]
    /// if the file is not present.
    /// ```no_run
    /// # use hf_hub::api::tokio::Api;
    /// # tokio_test::block_on(async {
    /// let api = Api::new().unwrap();
    /// let local_filename = api.model("gpt2".to_string()).get("model.safetensors").await.unwrap();
    /// # })
    pub async fn get(&self, filename: &FilePath) -> Result<Vec<TmpFileBlobKey>, ApiError> {
        log::info!("Checking/downloading file: {}", &filename.0);
        if let Some(_path) = self
            .api
            .cache
            .repo(self.repo.clone())
            .get(&self.api.db_client, filename)
            .await
            .unwrap()
        {
            // TODO: allow files to not only be chunks but also single blobs
            //
            // it's possible to always force "files" to be considered chunks, even if it's a single
            // chunk
            //
            // but this may be inneficient when loading the file to ram, unless there's a
            // verification during the loading
            unimplemented!()
            // Ok(path)
        } else {
            let res = self.download(filename).await;
            log::info!("Finished checking/downloading file: {}", &filename.0);
            res
        }
    }

    // TODO: rename function (get/download/etc)
    //
    pub async fn check(
        &self,
        filename: &FilePath,
        metadata: &Metadata,
    ) -> Result<TmpFileBlobKeyList, ApiError> {
        log::info!("Checking file: {}", &filename.0);
        // let url = self.url(filename);
        let cache = self.api.cache.repo(self.repo.clone());
        let blob_path: FileBlobKey = cache.blob_path(&metadata.etag);
        let size_bytes = metadata.size;

        let chunks_key_range = {
            use web_sys::IdbKeyRange;

            // 0-length starting at zero (there's no data before this)
            // [FileBlobKey]/{...000}-{...000}
            let before_first_chunk = blob_path.chunk(
                BlobChunkOffset(0),
                BlobChunkOffset(0),
                BlobChunkOffset(size_bytes),
            );
            // 0-length starting at the end (there's no data after this)
            // [FileBlobKey]/{size_bytes}-{size_bytes}
            let after_last_chunk = blob_path.chunk(
                BlobChunkOffset(size_bytes),
                BlobChunkOffset(size_bytes),
                BlobChunkOffset(size_bytes),
            );
            let before_first = JsValue::from_str(&before_first_chunk.0.join("/"));
            let after_last = JsValue::from_str(&after_last_chunk.0.join("/"));
            IdbKeyRange::bound(&before_first, &after_last).unwrap()
        };

        let cached_chunk_keys: Vec<TmpFileBlobKey> = self
            .api
            .cache
            .store_key_range(&self.api.db_client, DbStore::TmpFileBlob, chunks_key_range)
            .await
            .unwrap()
            .into_iter()
            .map(|s| TmpFileBlobKey::from_str(&s).unwrap())
            .collect();

        let mut chunks = vec![];
        let chunk_size = self.api.chunk_size;
        let mut byte_start = 0;
        let mut cached_chunk_i = 0;
        'outer: while byte_start < size_bytes {
            // log::info!("byte_start: {byte_start}, size_bytes: {size_bytes}");
            let mut byte_end = (byte_start + chunk_size).min(size_bytes);
            byte_end = match cached_chunk_keys.get(cached_chunk_i) {
                None => {
                    // log::info!("no more cached chunks");
                    // no more cached chunks available
                    byte_end
                }
                Some(next_cached_chunk) => {
                    use core::cmp::Ordering;

                    let (_prefix, cached_byte_start, cached_byte_end) = next_cached_chunk.split();
                    // log::info!("cached chunk info: cached_byte_start: {cached_byte_start:?}, cached_byte_end: {cached_byte_end:?}");
                    match byte_start.cmp(&cached_byte_start.0) {
                        Ordering::Less => {
                            // log::info!("cannot use the next cached chunk");
                            // cannot use the next cached chunk
                            // but still try to consider it for the chunk to be currently downloaded
                            byte_end.min(cached_byte_start.0)
                        }
                        Ordering::Equal => {
                            // log::info!("using the next cached chunk");
                            // can use the next cached chunk
                            chunks.push(Ok(next_cached_chunk.clone()));
                            cached_chunk_i += 1;
                            byte_start = cached_byte_end.0;
                            continue 'outer;
                        }
                        Ordering::Greater => {
                            // it's assumed that the byte offset to be downloaded
                            // cannot be higher than the start of the next cached chunk being verified
                            unreachable!()
                        }
                    }
                }
            };

            let byte_len = byte_end - byte_start;
            let chunk_file = blob_path.chunk(
                BlobChunkOffset(byte_start),
                BlobChunkOffset(byte_start + byte_len),
                BlobChunkOffset(size_bytes),
            );
            chunks.push(Err(chunk_file));
            byte_start += byte_len;
        }

        // let res = self.download(filename).await;
        // log::info!("Finished checking file: {}", &filename.0);
        // res
        Ok(chunks)
    }

    // TOO: rename function (get/download/etc)
    //
    /// Downloads a remote file (if not already present) into the cache directory
    /// to be used locally.
    /// This functions require internet access to verify if new versions of the file
    /// exist, even if a file is already on disk at location.
    /// ```no_run
    /// # use hf_hub::api::tokio::Api;
    /// # tokio_test::block_on(async {
    /// let api = Api::new().unwrap();
    /// let local_filename = api.model("gpt2".to_string()).download("model.safetensors").await.unwrap();
    /// # })
    /// ```
    pub async fn download(&self, filename: &FilePath) -> Result<Vec<TmpFileBlobKey>, ApiError> {
        let url = self.url(filename);
        let metadata: Metadata = self.api.metadata(&url).await.unwrap();
        let chunks = self.check(&filename, &metadata).await?;
        let tmp_filenames = self.download_tempfiles(chunks, &url).await.unwrap();

        // log::info!("Started/finished merging the chunks (currently unimplemented)");

        // TODO: merge temp files into bigger blobs (or even a single blob)
        //
        // Note: for web storage, this may not be desired as the browser may delete data arbitrarily,
        // and if the blobs are too big then too much data may get lost
        // eg. losing 20x 10MB blobs may be preferable than losing a single 1GB blob.
        //
        // Note: check how the merging can be done. Will the memory requirements be too high?
        // this matters because loading and storing blobs cannot be done in a streaming fashion

        // TODO: merge tmp files, save into the correct destination and then delete the tmp files

        // tokio::fs::rename(&tmp_filename, &blob_path).await.unwrap();

        // let mut pointer_path = cache.pointer_path(&metadata.commit_hash);
        // pointer_path.push(filename);
        // std::fs::create_dir_all(pointer_path.parent().unwrap()).ok();

        // symlink_or_rename(&blob_path, &pointer_path).unwrap();
        // cache.create_ref(&metadata.commit_hash).unwrap();

        // Ok(pointer_path)
        Ok(tmp_filenames)
    }

    // /// Get information about the Repo
    // /// ```
    // /// # use hf_hub::api::tokio::Api;
    // /// # tokio_test::block_on(async {
    // /// let api = Api::new().unwrap();
    // /// api.model("gpt2".to_string()).info();
    // /// # })
    // /// ```
    // pub async fn info(&self) -> Result<RepoInfo, ApiError> {
    //     Ok(self
    //         .info_request()
    //         .send()
    //         .await
    //         .unwrap()
    //         .json()
    //         .await
    //         .unwrap())
    // }

    // /// Get the raw [`reqwest::RequestBuilder`] with the url and method already set
    // /// ```
    // /// # use hf_hub::api::tokio::Api;
    // /// # tokio_test::block_on(async {
    // /// let api = Api::new().unwrap();
    // /// api.model("gpt2".to_owned())
    // ///     .info_request()
    // ///     .query(&[("blobs", "true")])
    // ///     .send()
    // ///     .await;
    // /// # })
    // /// ```
    // pub fn info_request(&self) -> RequestBuilder {
    //     let url = format!("{}/api/{}", self.api.endpoint, self.repo.api_url());
    //     self.api.client.get(url)
    // }
}
