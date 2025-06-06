// Conserve backup system.
// Copyright 2018-2023 Martin Pool.

// This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation; either version 2 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

//! Text output formats for structured data.
//!
//! These are objects that accept iterators of different types of content, and write it to a
//! file (typically stdout).

use std::borrow::Cow;
use std::io::{BufWriter, Write};
use std::sync::Arc;

use time::format_description::well_known::Rfc3339;
use time::UtcOffset;
use tracing::error;

use crate::index::entry::IndexEntry;
use crate::misc::duration_to_hms;
use crate::termui::TermUiMonitor;
use crate::*;

/// Options controlling the behavior of `show_versions`.
#[derive(Default, Clone, Eq, PartialEq)]
pub struct ShowVersionsOptions {
    /// Show versions in LIFO order by band_id.
    pub newest_first: bool,
    /// Show the total size of files in the tree.  This is
    /// slower because it requires walking the whole index.
    pub tree_size: bool,
    /// Show the date and time that each backup started.
    pub start_time: bool,
    /// Show how much time the backup took, or "incomplete" if it never finished.
    pub backup_duration: bool,
    /// Show times in this zone.
    pub timezone: Option<UtcOffset>,
}

/// Print a list of versions, one per line, on stdout.
pub async fn show_versions(
    archive: &Archive,
    options: &ShowVersionsOptions,
    monitor: Arc<TermUiMonitor>,
) -> Result<()> {
    let mut band_ids = archive.list_band_ids().await?;
    if options.newest_first {
        band_ids.reverse();
    }
    for band_id in band_ids {
        if !(options.tree_size || options.start_time || options.backup_duration) {
            println!("{band_id}");
            continue;
        }
        let mut l: Vec<String> = Vec::new();
        l.push(format!("{band_id:<20}"));
        let band = match Band::open(archive, band_id).await {
            Ok(band) => band,
            Err(err) => {
                error!("Failed to open band {band_id:?}: {err}");
                continue;
            }
        };
        let info = match band.get_info().await {
            Ok(info) => info,
            Err(err) => {
                error!("Failed to read band tail {band_id:?}: {err}");
                continue;
            }
        };

        if options.start_time {
            let mut start_time = info.start_time;
            if let Some(timezone) = options.timezone {
                start_time = start_time.to_offset(timezone);
            }
            l.push(format!(
                "{date:<25}", // "yyyy-mm-ddThh:mm:ss+oooo" => 25
                date = start_time.format(&Rfc3339).unwrap(),
            ));
        }

        if options.backup_duration {
            let duration_str: Cow<str> = if info.is_closed {
                if let Some(end_time) = info.end_time {
                    let duration = end_time - info.start_time;
                    if let Ok(duration) = duration.try_into() {
                        duration_to_hms(duration).into()
                    } else {
                        Cow::Borrowed("negative")
                    }
                } else {
                    Cow::Borrowed("unknown")
                }
            } else {
                Cow::Borrowed("incomplete")
            };
            l.push(format!("{duration_str:>10}"));
        }

        if options.tree_size {
            let sizes = archive
                .open_stored_tree(BandSelectionPolicy::Specified(band_id))
                .await?
                .size(Exclude::nothing(), monitor.clone())
                .await?;
            l.push(format!(
                "{:>14}",
                crate::misc::bytes_to_human_mb(sizes.file_bytes)
            ));
        }
        monitor.clear_progress_bars(); // to avoid fighting with stdout
        println!("{}", l.join(" "));
    }
    Ok(())
}

pub async fn show_index_json(band: &Band, w: &mut dyn Write) -> Result<()> {
    // TODO: Maybe use https://docs.serde.rs/serde/ser/trait.Serializer.html#method.collect_seq.
    let bw = BufWriter::new(w);
    let index_entries: Vec<Vec<IndexEntry>> = band
        .index()
        .iter_available_hunks()
        .await
        .collect_hunk_vec()
        .await?;
    serde_json::ser::to_writer_pretty(bw, &index_entries)
        .map_err(|source| Error::SerializeJson { source })
}
