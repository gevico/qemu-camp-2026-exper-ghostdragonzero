// Copyright 2026, QEMU Camp 2026 Experiment Team
// SPDX-License-Identifier: GPL-2.0-or-later

#[cfg(unix)]
use std::os::unix::fs::symlink as symlink_file;
#[cfg(windows)]
use std::os::windows::fs::symlink_file;
use std::{env, fs::remove_file, io::Result, path::Path};

fn main() -> Result<()> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let root = env::var("MESON_BUILD_ROOT").expect("MESON_BUILD_ROOT not found");
    let sub = manifest_dir.find("/rust").map(|index| &manifest_dir[index + 1..]).unwrap();
    let file = Path::new(&root).join(sub).join("bindings.inc.rs");

    if !file.exists() {
        panic!("No generated C bindings found. Run make first.");
    }

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("bindings.inc.rs");
    if dest_path.symlink_metadata().is_ok() {
        remove_file(&dest_path)?;
    }
    symlink_file(file, dest_path)?;

    println!("cargo:rerun-if-changed=build.rs");
    Ok(())
}
