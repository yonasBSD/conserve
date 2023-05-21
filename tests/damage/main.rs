// Conserve backup system.
// Copyright 2020-2023 Martin Pool.

// This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation; either version 2 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

use assert_fs::prelude::*;
use assert_fs::TempDir;
// use predicates::prelude::*;

use conserve::backup;
use conserve::restore;
use conserve::Archive;
use conserve::BackupOptions;
use conserve::RestoreOptions;
use dir_assert::assert_paths;
use rstest::rstest;
use tracing_test::traced_test;

mod strategy;
use strategy::Damage;

// TODO: Also test damage to other files: band tail, index hunks, data blocks, etc.
// TODO: Test that you can delete a damaged backup.

#[rstest]
#[traced_test]
#[test]
fn backup_after_damage(#[values(Damage::Delete, Damage::Truncate)] damage: Damage) {
    let archive_dir = TempDir::new().unwrap();
    let source_dir = TempDir::new().unwrap();

    let archive = Archive::create_path(archive_dir.path()).expect("create archive");
    source_dir
        .child("file")
        .write_str("content in first backup")
        .unwrap();

    let backup_options = BackupOptions::default();
    backup(&archive, source_dir.path(), &backup_options).expect("initial backup");

    damage.damage(&archive_dir.child("b0000").child("BANDHEAD"));

    // A second backup should succeed.
    source_dir
        .child("file")
        .write_str("content in second backup")
        .unwrap();
    backup(&archive, source_dir.path(), &backup_options)
        .expect("write second backup even though first bandhead is damaged");

    // Can restore the second backup
    let restore_dir = TempDir::new().unwrap();
    restore(&archive, restore_dir.path(), &RestoreOptions::default())
        .expect("restore second backup");

    // Since the second backup rewrote the single file in the backup (and the root dir),
    // we should get all the content back out.
    assert_paths!(source_dir.path(), restore_dir.path());

    // TODO: List versions.
    // TODO: List contents of second backup.
}
