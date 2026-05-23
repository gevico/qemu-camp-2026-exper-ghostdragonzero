// Copyright 2026 HUST OpenAtom Open Source Club.
// SPDX-License-Identifier: GPL-2.0-or-later

mod bindings;
mod bus;
mod device;
mod peripheral;
mod registers;

pub use device::{RustSpiState, TYPE_RUST_SPI};
pub use peripheral::{At25State, TYPE_AT25_RUST};
