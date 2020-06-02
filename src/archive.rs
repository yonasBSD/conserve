// Conserve backup system.
// Copyright 2015, 2016, 2017, 2018, 2019, 2020 Martin Pool.

//! Archives holding backup material.

use std::collections::BTreeSet;
use std::fs::read_dir;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::errors::Error;
use crate::io::file_exists;
use crate::jsonio;
use crate::misc::remove_item;
use crate::stats::ValidateArchiveStats;
use crate::*;

const HEADER_FILENAME: &str = "CONSERVE";
static BLOCK_DIR: &str = "d";

/// An archive holding backup material.
#[derive(Clone, Debug)]
pub struct Archive {
    /// Top-level directory for the archive.
    path: PathBuf,

    /// Holds body content for all file versions.
    block_dir: BlockDir,
}

#[derive(Debug, Serialize, Deserialize)]
struct ArchiveHeader {
    conserve_archive_version: String,
}

impl Archive {
    /// Make a new directory to hold an archive, and write the header.
    pub fn create(path: &Path) -> Result<Archive> {
        std::fs::create_dir(&path).map_err(|source| Error::CreateArchiveDirectory {
            path: path.to_owned(),
            source,
        })?;
        let block_dir = BlockDir::create(&path.join(BLOCK_DIR))?;
        let header = ArchiveHeader {
            conserve_archive_version: String::from(ARCHIVE_VERSION),
        };
        jsonio::write_json_metadata_file(&path.join(HEADER_FILENAME), &header)?;
        Ok(Archive {
            path: path.to_owned(),
            block_dir,
        })
    }

    /// Open an existing archive.
    ///
    /// Checks that the header is correct.
    pub fn open<P: Into<PathBuf>>(path: P) -> Result<Archive> {
        let path: PathBuf = path.into();
        let header_path = path.join(HEADER_FILENAME);
        if !file_exists(&header_path).map_err(|source| Error::ReadMetadata {
            path: path.to_owned(),
            source,
        })? {
            return Err(Error::NotAnArchive {
                path: path.to_owned(),
            });
        }
        let header: ArchiveHeader = jsonio::read_json_metadata_file(&header_path)?;
        if header.conserve_archive_version != ARCHIVE_VERSION {
            return Err(Error::UnsupportedArchiveVersion {
                version: header.conserve_archive_version,
                path,
            });
        }
        Ok(Archive {
            path: path.to_path_buf(),
            block_dir: BlockDir::new(&path.join(BLOCK_DIR)),
        })
    }

    pub fn block_dir(&self) -> &BlockDir {
        &self.block_dir
    }

    /// Returns the top-level directory for the archive.
    pub fn path(&self) -> &Path {
        self.path.as_path()
    }

    /// Returns a vector of band ids, in sorted order from first to last.
    pub fn list_bands(&self) -> Result<Vec<BandId>> {
        let mut band_ids = Vec::<BandId>::new();
        for e in read_dir(self.path())
            .map_err(|source| Error::ListBands {
                path: self.path.clone(),
                source,
            })?
            .filter_map(std::result::Result::ok)
        {
            if let Ok(n) = e.file_name().into_string() {
                if e.file_type().map(|ft| ft.is_dir()).unwrap_or(false) && n != BLOCK_DIR {
                    band_ids.push(n.parse()?)
                }
            }
            // TODO: Log errors while reading the directory.
        }
        band_ids.sort_unstable();
        Ok(band_ids)
    }

    /// Return the `BandId` of the highest-numbered band, or Ok(None) if there
    /// are no bands, or an Err if any occurred reading the directory.
    pub fn last_band_id(&self) -> Result<Option<BandId>> {
        // TODO: Perhaps factor out an iter_bands_unsorted, common
        // between this and list_bands.
        Ok(self.list_bands()?.into_iter().last())
    }

    /// Return the last completely-written band id, if any.
    pub fn last_complete_band(&self) -> Result<Option<Band>> {
        for id in self.list_bands()?.iter().rev() {
            let b = Band::open(self, &id)?;
            if b.is_closed()? {
                return Ok(Some(b));
            }
        }
        Ok(None)
    }

    /// Return a sorted set containing all the blocks referenced by all bands.
    pub fn referenced_blocks(&self) -> Result<BTreeSet<String>> {
        let mut hs = BTreeSet::<String>::new();
        for band_id in self.list_bands()? {
            let band = Band::open(&self, &band_id)?;
            for ie in band.iter_entries()? {
                for a in ie.addrs {
                    hs.insert(a.hash);
                }
            }
        }
        Ok(hs)
    }

    pub fn validate(&self) -> Result<ValidateArchiveStats> {
        let mut stats = self.validate_archive_dir()?;
        ui::println("Check blockdir...");
        stats.block_dir += self.block_dir.validate()?;
        self.validate_bands(&mut stats)?;

        if stats.has_problems() {
            ui::problem("Archive has some problems.");
        } else {
            ui::println("Archive is OK.");
        }
        Ok(stats)
    }

    fn validate_archive_dir(&self) -> Result<ValidateArchiveStats> {
        // TODO: Tests for the problems detected here.
        let mut stats = ValidateArchiveStats::default();
        ui::println("Check archive top-level directory...");
        let (mut files, mut dirs) =
            list_dir(self.path()).map_err(|source| Error::ReadMetadata {
                source,
                path: self.path().to_owned(),
            })?;
        remove_item(&mut files, &HEADER_FILENAME);
        if !files.is_empty() {
            stats.structure_problems += 1;
            ui::problem(&format!(
                "Unexpected files in archive directory {:?}: {:?}",
                self.path(),
                files
            ));
        }
        remove_item(&mut dirs, &BLOCK_DIR);
        dirs.sort();
        let mut bs = BTreeSet::<BandId>::new();
        for d in dirs.iter() {
            if let Ok(b) = d.parse() {
                if bs.contains(&b) {
                    stats.structure_problems += 1;
                    ui::problem(&format!(
                        "Duplicated band directory in {:?}: {:?}",
                        self.path(),
                        d
                    ));
                } else {
                    bs.insert(b);
                }
            } else {
                stats.structure_problems += 1;
                ui::problem(&format!(
                    "Unexpected directory in {:?}: {:?}",
                    self.path(),
                    d
                ));
            }
        }
        Ok(stats)
    }

    fn validate_bands(&self, _stats: &mut ValidateArchiveStats) -> Result<()> {
        // TODO: Don't stop early on any errors in the steps below, but do count them.
        // TODO: Better progress bars, that don't work by size but rather by
        // count.
        // TODO: Take in a dict of the known blocks and their decompressed lengths,
        // and use that to more cheaply check if the index is OK.
        use crate::ui::println;
        ui::clear_bytes_total();
        for bid in self.list_bands()?.iter() {
            println(&format!("Check {}...", bid));
            let b = Band::open(self, bid)?;
            b.validate()?;

            let st = StoredTree::open_incomplete_version(self, bid)?;
            st.validate()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::Read;
    use tempfile::TempDir;

    use super::*;
    use crate::test_fixtures::ScratchArchive;

    #[test]
    fn create_then_open_archive() {
        let testdir = TempDir::new().unwrap();
        let arch_path = testdir.path().join("arch");
        let arch = Archive::create(&arch_path).unwrap();

        assert_eq!(arch.path(), arch_path.as_path());
        assert!(arch.list_bands().unwrap().is_empty());

        // We can re-open it.
        Archive::open(arch_path).unwrap();
        assert!(arch.list_bands().unwrap().is_empty());
        assert!(arch.last_complete_band().unwrap().is_none());
    }

    /// A new archive contains just one header file.
    /// The header is readable json containing only a version number.
    #[test]
    fn empty_archive() {
        let af = ScratchArchive::new();
        let (file_names, dir_names) = list_dir(af.path()).unwrap();
        assert_eq!(file_names, &["CONSERVE"]);
        assert_eq!(dir_names, &["d"]);

        let header_path = af.path().join("CONSERVE");
        let mut header_file = fs::File::open(&header_path).unwrap();
        let mut contents = String::new();
        header_file.read_to_string(&mut contents).unwrap();
        assert_eq!(contents, "{\"conserve_archive_version\":\"0.6\"}\n");

        assert!(
            af.last_band_id().unwrap().is_none(),
            "Archive should have no bands yet"
        );
        assert!(
            af.last_complete_band().unwrap().is_none(),
            "Archive should have no bands yet"
        );
        assert!(af.referenced_blocks().unwrap().is_empty());
        assert_eq!(af.block_dir.block_names().unwrap().count(), 0);
    }

    #[test]
    fn create_bands() {
        let af = ScratchArchive::new();

        // Make one band
        let _band1 = Band::create(&af).unwrap();
        let (_file_names, dir_names) = list_dir(af.path()).unwrap();
        assert_eq!(dir_names, &["b0000", "d"]);

        assert_eq!(af.list_bands().unwrap(), vec![BandId::new(&[0])]);
        assert_eq!(af.last_band_id().unwrap(), Some(BandId::new(&[0])));

        // Try creating a second band.
        let _band2 = Band::create(&af).unwrap();
        assert_eq!(
            af.list_bands().unwrap(),
            vec![BandId::new(&[0]), BandId::new(&[1])]
        );
        assert_eq!(af.last_band_id().unwrap(), Some(BandId::new(&[1])));

        assert!(af.referenced_blocks().unwrap().is_empty());
        assert_eq!(af.block_dir.block_names().unwrap().count(), 0);
    }
}
