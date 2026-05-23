// SPDX-License-Identifier: GPL-2.0-or-later

#[cfg(not(test))]
use std::{cell::UnsafeCell, ffi::CStr};

#[cfg(not(test))]
use common::prelude::*;
#[cfg(not(test))]
use hwcore::prelude::*;
#[cfg(not(test))]
use qom::prelude::*;

#[cfg(not(test))]
use crate::bus::{I2CEvent, I2CSlave, I2CSlaveImpl};

#[cfg(not(test))]
pub const TYPE_AT24C02_RUST: &CStr = c"at24c02-rust";

#[cfg(not(test))]
const AT24C02_SIZE: usize = 256;
#[cfg(not(test))]
const AT24C02_PAGE_SIZE: u8 = 8;

#[cfg(not(test))]
#[repr(C)]
#[derive(qom::Object, hwcore::Device)]
pub struct At24c02State {
    pub parent_obj: ParentField<I2CSlave>,
    storage: UnsafeCell<[u8; AT24C02_SIZE]>,
    pointer: UnsafeCell<u8>,
    first_byte: UnsafeCell<bool>,
    page_base: UnsafeCell<u8>,
    page_offset: UnsafeCell<u8>,
}

#[cfg(not(test))]
unsafe impl Send for At24c02State {}
#[cfg(not(test))]
unsafe impl Sync for At24c02State {}

#[cfg(not(test))]
impl Default for At24c02State {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

#[cfg(not(test))]
qom_isa!(At24c02State : I2CSlave, DeviceState, Object);

#[cfg(not(test))]
#[repr(C)]
pub struct At24c02Class {
    parent_class: <I2CSlave as ObjectType>::Class,
}

#[cfg(not(test))]
trait At24c02Impl: I2CSlaveImpl + IsA<At24c02State> {}

#[cfg(not(test))]
impl At24c02Impl for At24c02State {}

#[cfg(not(test))]
impl At24c02Class {
    fn class_init<T: At24c02Impl>(&mut self) {
        self.parent_class.class_init::<T>();
    }
}

#[cfg(not(test))]
unsafe impl ObjectType for At24c02State {
    type Class = At24c02Class;
    const TYPE_NAME: &'static CStr = TYPE_AT24C02_RUST;
}

#[cfg(not(test))]
impl At24c02State {
    pub unsafe fn init(mut this: ParentInit<At24c02State>) {
        uninit_field_mut!(*this, storage).write(UnsafeCell::new([0xFF; AT24C02_SIZE]));
        uninit_field_mut!(*this, pointer).write(UnsafeCell::new(0));
        uninit_field_mut!(*this, first_byte).write(UnsafeCell::new(true));
        uninit_field_mut!(*this, page_base).write(UnsafeCell::new(0));
        uninit_field_mut!(*this, page_offset).write(UnsafeCell::new(0));
    }

    fn reset_hold(&self, _typ: ResetType) {
        unsafe {
            *self.storage.get() = [0xFF; AT24C02_SIZE];
            *self.pointer.get() = 0;
            *self.first_byte.get() = true;
            *self.page_base.get() = 0;
            *self.page_offset.get() = 0;
        }
    }
}

#[cfg(not(test))]
impl ObjectImpl for At24c02State {
    type ParentType = I2CSlave;

    const INSTANCE_INIT: Option<unsafe fn(ParentInit<Self>)> = Some(Self::init);
    const CLASS_INIT: fn(&mut Self::Class) = Self::Class::class_init::<Self>;
}

#[cfg(not(test))]
impl DeviceImpl for At24c02State {}

#[cfg(not(test))]
impl ResettablePhasesImpl for At24c02State {
    const HOLD: Option<fn(&Self, ResetType)> = Some(Self::reset_hold);
}

#[cfg(not(test))]
impl I2CSlaveImpl for At24c02State {
    fn event(&self, event: I2CEvent) -> i32 {
        if event == I2CEvent::StartSend {
            unsafe {
                *self.first_byte.get() = true;
            }
        }
        0
    }

    fn send(&self, data: u8) -> i32 {
        unsafe {
            if *self.first_byte.get() {
                *self.pointer.get() = data;
                *self.page_base.get() = data & !(AT24C02_PAGE_SIZE - 1);
                *self.page_offset.get() = data & (AT24C02_PAGE_SIZE - 1);
                *self.first_byte.get() = false;
                return 0;
            }

            let index = *self.page_base.get() | *self.page_offset.get();
            (*self.storage.get())[index as usize] = data;
            *self.pointer.get() = index.wrapping_add(1);
            *self.page_offset.get() = (*self.page_offset.get() + 1) & (AT24C02_PAGE_SIZE - 1);
        }
        0
    }

    fn recv(&self) -> u8 {
        unsafe {
            let value = (*self.storage.get())[*self.pointer.get() as usize];
            *self.pointer.get() = (*self.pointer.get()).wrapping_add(1);
            value
        }
    }
}
