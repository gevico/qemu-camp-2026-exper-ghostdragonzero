// Copyright 2026 HUST OpenAtom Open Source Club.
// SPDX-License-Identifier: GPL-2.0-or-later

pub const RSPI_SIZE: u64 = 0x1000;

pub mod offset {
    pub const CR1: u32 = 0x00;
    pub const SR: u32 = 0x04;
    pub const DR: u32 = 0x08;
    pub const CS: u32 = 0x0C;
}

pub mod cr1 {
    pub const SPE: u32 = 1 << 0;
    pub const MSTR: u32 = 1 << 2;
}

pub mod sr {
    pub const RXNE: u32 = 1 << 0;
    pub const TXE: u32 = 1 << 1;
    pub const OVERRUN: u32 = 1 << 4;
}
