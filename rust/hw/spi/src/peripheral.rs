// SPDX-License-Identifier: GPL-2.0-or-later

use std::{cell::UnsafeCell, ffi::CStr};

use common::prelude::*;
use hwcore::prelude::*;
use qom::prelude::*;

use crate::bus::{SSIPeripheral, SSIPeripheralImpl};

pub const TYPE_AT25_RUST: &CStr = c"at25-rust";

const AT25_SIZE: usize = 256;
const AT25_CMD_WREN: u8 = 0x06;
const AT25_CMD_RDSR: u8 = 0x05;
const AT25_CMD_READ: u8 = 0x03;
const AT25_CMD_WRITE: u8 = 0x02;
const AT25_SR_WEL: u8 = 1 << 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FlashMode {
    Idle,
    ReadStatus,
    ReadAddr,
    ReadData,
    WriteAddr,
    WriteData,
}

#[derive(Debug)]
struct At25Core {
    storage: [u8; AT25_SIZE],
    status: u8,
    mode: FlashMode,
    pointer: u8,
    write_count: u8,
}

impl At25Core {
    fn new() -> Self {
        Self {
            storage: [0xFF; AT25_SIZE],
            status: 0,
            mode: FlashMode::Idle,
            pointer: 0,
            write_count: 0,
        }
    }

    fn start_command(&mut self, tx: u8) -> u8 {
        match tx {
            AT25_CMD_WREN => {
                self.status |= AT25_SR_WEL;
                self.mode = FlashMode::Idle;
                0
            }
            AT25_CMD_RDSR => {
                self.mode = FlashMode::ReadStatus;
                0
            }
            AT25_CMD_READ => {
                self.mode = FlashMode::ReadAddr;
                0
            }
            AT25_CMD_WRITE => {
                self.mode = FlashMode::WriteAddr;
                self.write_count = 0;
                0
            }
            _ => tx,
        }
    }

    fn transfer(&mut self, tx: u8) -> u8 {
        match self.mode {
            FlashMode::Idle => self.start_command(tx),
            FlashMode::ReadStatus => {
                self.mode = FlashMode::Idle;
                self.status
            }
            FlashMode::ReadAddr => {
                self.pointer = tx;
                self.mode = FlashMode::ReadData;
                0
            }
            FlashMode::ReadData => {
                let value = self.storage[self.pointer as usize];
                self.pointer = self.pointer.wrapping_add(1);
                value
            }
            FlashMode::WriteAddr => {
                self.pointer = tx;
                self.mode = FlashMode::WriteData;
                0
            }
            FlashMode::WriteData => {
                if self.write_count > 0
                    && matches!(tx, AT25_CMD_WREN | AT25_CMD_RDSR | AT25_CMD_READ | AT25_CMD_WRITE)
                {
                    self.status &= !AT25_SR_WEL;
                    return self.start_command(tx);
                }

                if self.status & AT25_SR_WEL != 0 {
                    self.storage[self.pointer as usize] = tx;
                    self.pointer = self.pointer.wrapping_add(1);
                    self.write_count = self.write_count.wrapping_add(1);
                }
                0
            }
        }
    }
}

#[repr(C)]
#[derive(qom::Object, hwcore::Device)]
pub struct At25State {
    pub parent_obj: ParentField<SSIPeripheral>,
    core: UnsafeCell<At25Core>,
}

unsafe impl Send for At25State {}
unsafe impl Sync for At25State {}

impl Default for At25State {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

qom_isa!(At25State : SSIPeripheral, DeviceState, Object);

#[repr(C)]
pub struct At25Class {
    parent_class: <SSIPeripheral as ObjectType>::Class,
}

trait At25Impl: SSIPeripheralImpl + IsA<At25State> {}

impl At25Impl for At25State {}

impl At25Class {
    fn class_init<T: At25Impl>(&mut self) {
        self.parent_class.class_init::<T>();
    }
}

unsafe impl ObjectType for At25State {
    type Class = At25Class;
    const TYPE_NAME: &'static CStr = TYPE_AT25_RUST;
}

impl At25State {
    pub unsafe fn init(mut this: ParentInit<At25State>) {
        uninit_field_mut!(*this, core).write(UnsafeCell::new(At25Core::new()));
    }

    fn reset_hold(&self, _typ: ResetType) {
        unsafe {
            *self.core.get() = At25Core::new();
        }
    }
}

impl ObjectImpl for At25State {
    type ParentType = SSIPeripheral;

    const INSTANCE_INIT: Option<unsafe fn(ParentInit<Self>)> = Some(Self::init);
    const CLASS_INIT: fn(&mut Self::Class) = Self::Class::class_init::<Self>;
}

impl DeviceImpl for At25State {}

impl ResettablePhasesImpl for At25State {
    const HOLD: Option<fn(&Self, ResetType)> = Some(Self::reset_hold);
}

impl SSIPeripheralImpl for At25State {
    fn transfer(&self, value: u32) -> u32 {
        unsafe { (*self.core.get()).transfer((value & 0xFF) as u8) as u32 }
    }
}
