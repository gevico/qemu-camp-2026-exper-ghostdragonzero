// SPDX-License-Identifier: GPL-2.0-or-later

//! SSI bus and peripheral abstractions.
//!
//! This mirrors QEMU's `hw/ssi/ssi.c`: `SSIBus` is a `BusState`
//! subclass and `SSIPeripheral` is a `DeviceState` subclass whose
//! class provides `realize`, `transfer`, `set_cs`, and `transfer_raw`
//! callbacks.

use std::{ffi::CStr, os::raw::c_int, ptr::NonNull};

use common::Opaque;
use hwcore::prelude::*;
use qom::prelude::*;

use crate::bindings;

pub const TYPE_SSI_BUS: &CStr = c"SSI";

#[repr(transparent)]
#[derive(Debug, common::Wrapper)]
pub struct SSIBus(Opaque<bindings::SSIBus>);

unsafe impl Send for SSIBus {}
unsafe impl Sync for SSIBus {}

unsafe impl ObjectType for SSIBus {
    type Class = hwcore::bindings::BusClass;
    const TYPE_NAME: &'static CStr = TYPE_SSI_BUS;
}

qom_isa!(SSIBus: BusState, Object);

impl SSIBus {
    pub unsafe fn create(parent: *mut hwcore::bindings::DeviceState, name: &CStr) -> *mut Self {
        unsafe { bindings::ssi_create_bus(parent, name.as_ptr()).cast() }
    }

    pub fn transfer(&self, value: u32) -> u32 {
        unsafe { bindings::ssi_transfer(self.as_mut_ptr(), value) }
    }
}

#[repr(transparent)]
#[derive(Debug, common::Wrapper)]
pub struct SSIPeripheral(Opaque<bindings::SSIPeripheral>);

unsafe impl Send for SSIPeripheral {}
unsafe impl Sync for SSIPeripheral {}

qom_isa!(SSIPeripheral: DeviceState, Object);

#[repr(transparent)]
pub struct SSIPeripheralClass(bindings::SSIPeripheralClass);

pub trait SSIPeripheralImpl: DeviceImpl + IsA<SSIPeripheral> {
    fn transfer(&self, value: u32) -> u32;

    fn realize(&self) {}

    fn set_cs(&self, select: bool) -> i32 {
        let _ = select;
        0
    }
}

unsafe extern "C" fn rust_ssi_realize<T: SSIPeripheralImpl>(
    dev: *mut bindings::SSIPeripheral,
    _errp: *mut *mut util::bindings::Error,
) {
    let state = NonNull::new(dev).unwrap().cast::<T>();
    T::realize(unsafe { state.as_ref() });
}

unsafe extern "C" fn rust_ssi_transfer<T: SSIPeripheralImpl>(
    dev: *mut bindings::SSIPeripheral,
    value: u32,
) -> u32 {
    let state = NonNull::new(dev).unwrap().cast::<T>();
    T::transfer(unsafe { state.as_ref() }, value)
}

unsafe extern "C" fn rust_ssi_set_cs<T: SSIPeripheralImpl>(
    dev: *mut bindings::SSIPeripheral,
    select: bool,
) -> c_int {
    let state = NonNull::new(dev).unwrap().cast::<T>();
    T::set_cs(unsafe { state.as_ref() }, select) as c_int
}

impl SSIPeripheralClass {
    pub fn class_init<T: SSIPeripheralImpl>(&mut self) {
        self.0.parent_class.class_init::<T>();
        self.0.realize = Some(rust_ssi_realize::<T>);
        self.0.transfer = Some(rust_ssi_transfer::<T>);
        self.0.set_cs = Some(rust_ssi_set_cs::<T>);
        self.0.cs_polarity = bindings::SSI_CS_NONE;
    }
}

unsafe impl ObjectType for SSIPeripheral {
    type Class = SSIPeripheralClass;
    const TYPE_NAME: &'static CStr =
        unsafe { CStr::from_bytes_with_nul_unchecked(bindings::TYPE_SSI_PERIPHERAL) };
}
