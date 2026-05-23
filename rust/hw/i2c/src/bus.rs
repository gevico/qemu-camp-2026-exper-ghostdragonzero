// SPDX-License-Identifier: GPL-2.0-or-later

//! I2C bus and slave abstractions.
//!
//! The non-test implementation mirrors QEMU's C I2C model from
//! `include/hw/i2c/i2c.h`: an `I2CBus` is a `BusState`, an `I2CSlave`
//! is a `DeviceState`, and concrete Rust slaves implement virtual
//! callbacks equivalent to `I2CSlaveClass`.

#[cfg(not(test))]
use std::{
    ffi::CStr,
    os::raw::c_int,
    ptr::{addr_of_mut, NonNull},
};

#[cfg(not(test))]
use common::Opaque;
#[cfg(not(test))]
use hwcore::prelude::*;
#[cfg(not(test))]
use qom::prelude::*;

#[cfg(not(test))]
use crate::bindings;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum I2CEvent {
    StartRecv,
    StartSend,
    #[cfg(not(test))]
    StartSendAsync,
    Finish,
    Nack,
}

#[cfg(test)]
pub trait I2CSlave {
    fn address(&self) -> u8;

    fn event(&mut self, event: I2CEvent) -> i32 {
        let _ = event;
        0
    }

    fn send(&mut self, data: u8) -> i32;

    fn recv(&mut self) -> u8;
}

#[cfg(test)]
pub struct I2CBus {
    devices: Vec<Box<dyn I2CSlave>>,
    current_addr: Option<u8>,
    is_recv: bool,
}

#[cfg(test)]
impl std::fmt::Debug for I2CBus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("I2CBus")
            .field("device_count", &self.devices.len())
            .field("current_addr", &self.current_addr)
            .field("is_recv", &self.is_recv)
            .finish()
    }
}

#[cfg(test)]
impl I2CBus {
    pub fn new() -> Self {
        Self {
            devices: Vec::new(),
            current_addr: None,
            is_recv: false,
        }
    }

    pub fn attach(&mut self, device: Box<dyn I2CSlave>) {
        self.devices.push(device);
    }

    pub fn device_count(&self) -> usize {
        self.devices.len()
    }

    pub fn is_busy(&self) -> bool {
        self.current_addr.is_some()
    }

    pub fn start_transfer(&mut self, address: u8, is_recv: bool) -> i32 {
        for device in &mut self.devices {
            if device.address() == address {
                let event = if is_recv {
                    I2CEvent::StartRecv
                } else {
                    I2CEvent::StartSend
                };

                if device.event(event) == 0 {
                    self.current_addr = Some(address);
                    self.is_recv = is_recv;
                    return 0;
                }
            }
        }
        -1
    }

    pub fn end_transfer(&mut self) {
        if let Some(addr) = self.current_addr {
            for device in &mut self.devices {
                if device.address() == addr {
                    device.event(I2CEvent::Finish);
                    break;
                }
            }
            self.current_addr = None;
        }
    }

    pub fn send(&mut self, data: u8) -> i32 {
        if let Some(addr) = self.current_addr {
            for device in &mut self.devices {
                if device.address() == addr {
                    return device.send(data);
                }
            }
        }
        -1
    }

    pub fn recv(&mut self) -> u8 {
        if let Some(addr) = self.current_addr {
            for device in &mut self.devices {
                if device.address() == addr {
                    return device.recv();
                }
            }
        }
        0xFF
    }

    pub fn transfer_write(&mut self, addr: u8, data: &[u8]) -> bool {
        if self.start_transfer(addr, false) != 0 {
            return false;
        }
        for &byte in data {
            if self.send(byte) != 0 {
                self.end_transfer();
                return false;
            }
        }
        self.end_transfer();
        true
    }

    pub fn transfer_read(&mut self, addr: u8, len: usize) -> Option<Vec<u8>> {
        if self.start_transfer(addr, true) != 0 {
            return None;
        }
        let mut result = Vec::with_capacity(len);
        for _ in 0..len {
            result.push(self.recv());
        }
        self.end_transfer();
        Some(result)
    }
}

#[cfg(test)]
impl Default for I2CBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(test))]
impl I2CEvent {
    fn from_raw(event: bindings::i2c_event) -> Self {
        match event {
            bindings::I2C_START_RECV => Self::StartRecv,
            bindings::I2C_START_SEND => Self::StartSend,
            bindings::I2C_START_SEND_ASYNC => Self::StartSendAsync,
            bindings::I2C_FINISH => Self::Finish,
            bindings::I2C_NACK => Self::Nack,
            _ => Self::Nack,
        }
    }
}

#[cfg(not(test))]
#[repr(transparent)]
#[derive(Debug, common::Wrapper)]
pub struct I2CBus(Opaque<bindings::I2CBus>);

#[cfg(not(test))]
unsafe impl Send for I2CBus {}
#[cfg(not(test))]
unsafe impl Sync for I2CBus {}

#[cfg(not(test))]
unsafe impl ObjectType for I2CBus {
    type Class = hwcore::bindings::BusClass;
    const TYPE_NAME: &'static CStr =
        unsafe { CStr::from_bytes_with_nul_unchecked(bindings::TYPE_I2C_BUS) };
}

#[cfg(not(test))]
qom_isa!(I2CBus: BusState, Object);

#[cfg(not(test))]
impl I2CBus {
    pub unsafe fn init_bus(parent: *mut hwcore::bindings::DeviceState, name: &CStr) -> *mut Self {
        unsafe { bindings::i2c_init_bus(parent, name.as_ptr()).cast() }
    }

    pub fn is_busy(&self) -> bool {
        unsafe { bindings::i2c_bus_busy(self.as_mut_ptr()) != 0 }
    }

    pub fn start_transfer(&self, address: u8, is_recv: bool) -> i32 {
        unsafe { bindings::i2c_start_transfer(self.as_mut_ptr(), address, is_recv) }
    }

    pub fn send(&self, data: u8) -> i32 {
        unsafe { bindings::i2c_send(self.as_mut_ptr(), data) }
    }

    pub fn recv(&self) -> u8 {
        unsafe { bindings::i2c_recv(self.as_mut_ptr()) }
    }

    pub fn end_transfer(&self) {
        unsafe { bindings::i2c_end_transfer(self.as_mut_ptr()) }
    }

    pub fn nack(&self) {
        unsafe { bindings::i2c_nack(self.as_mut_ptr()) }
    }
}

#[cfg(not(test))]
#[repr(transparent)]
#[derive(Debug, common::Wrapper)]
pub struct I2CSlave(Opaque<bindings::I2CSlave>);

#[cfg(not(test))]
unsafe impl Send for I2CSlave {}
#[cfg(not(test))]
unsafe impl Sync for I2CSlave {}

#[cfg(not(test))]
qom_isa!(I2CSlave: DeviceState, Object);

#[cfg(not(test))]
#[repr(transparent)]
pub struct I2CSlaveClass(bindings::I2CSlaveClass);

#[cfg(not(test))]
pub trait I2CSlaveImpl: DeviceImpl + IsA<I2CSlave> {
    fn send(&self, data: u8) -> i32;
    fn recv(&self) -> u8;

    fn event(&self, event: I2CEvent) -> i32 {
        let _ = event;
        0
    }
}

#[cfg(not(test))]
unsafe extern "C" fn rust_i2c_slave_send<T: I2CSlaveImpl>(
    dev: *mut bindings::I2CSlave,
    data: u8,
) -> c_int {
    let state = NonNull::new(dev).unwrap().cast::<T>();
    T::send(unsafe { state.as_ref() }, data) as c_int
}

#[cfg(not(test))]
unsafe extern "C" fn rust_i2c_slave_recv<T: I2CSlaveImpl>(dev: *mut bindings::I2CSlave) -> u8 {
    let state = NonNull::new(dev).unwrap().cast::<T>();
    T::recv(unsafe { state.as_ref() })
}

#[cfg(not(test))]
unsafe extern "C" fn rust_i2c_slave_event<T: I2CSlaveImpl>(
    dev: *mut bindings::I2CSlave,
    event: bindings::i2c_event,
) -> c_int {
    let state = NonNull::new(dev).unwrap().cast::<T>();
    T::event(unsafe { state.as_ref() }, I2CEvent::from_raw(event)) as c_int
}

#[cfg(not(test))]
unsafe extern "C" fn rust_i2c_slave_match_and_add(
    candidate: *mut bindings::I2CSlave,
    address: u8,
    broadcast: bool,
    current_devs: *mut bindings::I2CNodeList,
) -> bool {
    unsafe {
        if (*candidate).address != address && !broadcast {
            return false;
        }

        let node = glib_sys::g_malloc(std::mem::size_of::<bindings::I2CNode>())
            .cast::<bindings::I2CNode>();
        (*node).elt = candidate;
        (*node).next.le_next = (*current_devs).lh_first;
        if !(*current_devs).lh_first.is_null() {
            (*(*current_devs).lh_first).next.le_prev = addr_of_mut!((*node).next.le_next);
        }
        (*current_devs).lh_first = node;
        (*node).next.le_prev = addr_of_mut!((*current_devs).lh_first);
        true
    }
}

#[cfg(not(test))]
impl I2CSlaveClass {
    pub fn class_init<T: I2CSlaveImpl>(&mut self) {
        self.0.parent_class.class_init::<T>();
        self.0.parent_class.bus_type = bindings::TYPE_I2C_BUS.as_ptr().cast();
        self.0.send = Some(rust_i2c_slave_send::<T>);
        self.0.recv = Some(rust_i2c_slave_recv::<T>);
        self.0.event = Some(rust_i2c_slave_event::<T>);
        self.0.match_and_add = Some(rust_i2c_slave_match_and_add);
    }
}

#[cfg(not(test))]
unsafe impl ObjectType for I2CSlave {
    type Class = I2CSlaveClass;
    const TYPE_NAME: &'static CStr =
        unsafe { CStr::from_bytes_with_nul_unchecked(bindings::TYPE_I2C_SLAVE) };
}
