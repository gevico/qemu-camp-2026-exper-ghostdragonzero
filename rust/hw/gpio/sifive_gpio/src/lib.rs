// Copyright 2024, Linaro Limited
// Author(s): Manos Pitsidianakis <manos.pitsidianakis@linaro.org>
// SPDX-License-Identifier: GPL-2.0-or-later

//! PL011 QEMU Device Model
//!
//! This library implements a device model for the PrimeCell® UART (PL011)
//! device in QEMU.
//!
//! # Library crate
//!
//! See [`PL011State`](crate::device::PL011State) for the device model type and
//! the [`registers`] module for register types.

mod bindings;
mod device;
mod registers;

// Device will be created through QEMU's standard device system using TYPE_SIFIVE_GPIO
pub const TYPE_SIFIVE_GPIO: &::std::ffi::CStr = c"sifive_soc.gpio";
