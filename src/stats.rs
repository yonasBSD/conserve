// Conserve backup system.
// Copyright 2015, 2016, 2017, 2018, 2019, 2020 Martin Pool.

use derive_more::{Add, AddAssign};

/// Describes sizes of data read or written, with both the
/// compressed and uncompressed size.
#[derive(Add, AddAssign, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Sizes {
    pub compressed: u64,
    pub uncompressed: u64,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ValidateArchiveStats {
    pub block_dir_stats: ValidateBlockDirStats,
}

#[derive(Clone, Default, Debug, Eq, PartialEq)]
pub struct ValidateBlockDirStats {
    /// Number of blocks read.
    pub block_read_count: u64,
    /// Number of blocks that failed to read back.
    pub block_error_count: u64,
}

#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct IndexEntryIterStats {
    pub index_hunks: u64,
    pub uncompressed_index_bytes: u64,
    pub compressed_index_bytes: u64,
}

#[derive(Add, AddAssign, Clone, Debug, Default, Eq, PartialEq)]
pub struct IndexBuilderStats {
    pub index_hunks: u64,
    pub uncompressed_index_bytes: u64,
    pub compressed_index_bytes: u64,
}

#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct LiveTreeIterStats {
    pub directories_visited: usize,
    pub exclusions: usize,
    pub metadata_error: usize,
    pub entries_returned: usize,
}

#[derive(Add, AddAssign, Debug, Default, Eq, PartialEq, Clone)]
pub struct CopyStats {
    pub files: usize,
    pub symlinks: usize,
    pub directories: usize,
    pub unknown_kind: usize,

    pub files_unmodified: usize,
    pub files_modified: usize,
    pub files_new: usize,

    pub deduplicated_bytes: u64,
    pub uncompressed_bytes: u64,
    pub compressed_bytes: u64,

    pub deduplicated_blocks: usize,
    pub written_blocks: usize,

    pub empty_files: usize,
    pub single_block_files: usize,
    pub multi_block_files: usize,

    pub errors: usize,

    pub index_builder_stats: IndexBuilderStats,
}