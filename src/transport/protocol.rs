// Copyright 2020-2025 Martin Pool.

//! Transport protocol for reading and writing files, abstracting across local and various remote filesystems.
//!
//! This isn't exposed in the public API but is only an internal detail of the transport module.

use std::path::PathBuf;
use std::sync::Arc;

use bytes::Bytes;
use url::Url;

use super::{ListDir, Metadata, Result, WriteMode};

pub(super) trait Protocol: std::fmt::Debug + Send + Sync {
    fn read(&self, path: &str) -> Result<Bytes>;

    /// Write a complete file.
    ///
    /// Depending on the [WriteMode] this may either overwrite existing files, or error.
    ///
    /// As much as possible, the file should be written atomically so that it is only visible with
    /// the complete content.
    fn write(&self, relpath: &str, content: &[u8], mode: WriteMode) -> Result<()>;
    fn list_dir(&self, relpath: &str) -> Result<ListDir>;
    fn create_dir(&self, relpath: &str) -> Result<()>;

    /// Get metadata about a file.
    fn metadata(&self, relpath: &str) -> Result<Metadata>;

    /// Delete a file.
    fn remove_file(&self, relpath: &str) -> Result<()>;

    /// Delete a directory and all its contents.
    fn remove_dir_all(&self, relpath: &str) -> Result<()>;

    /// Make a new transport addressing a subdirectory.
    fn chdir(&self, relpath: &str) -> Arc<dyn Protocol>;

    fn url(&self) -> &Url;

    fn local_path(&self) -> Option<PathBuf> {
        None
    }
}
