// Copyright 2025 HUST OpenAtom Open Source Club.
// SPDX-License-Identifier: GPL-2.0-or-later

//! I2C controller device implementation.
use std::ffi::CStr;
#[cfg(not(test))]
use std::{cell::UnsafeCell, mem::MaybeUninit};

#[cfg(not(test))]
use common::prelude::*;
#[cfg(not(test))]
use hwcore::prelude::*;
#[cfg(not(test))]
use qom::prelude::*;
#[cfg(not(test))]
use system::prelude::*;

use crate::bus::{I2CBus, I2CEvent, I2CSlave};
use crate::register::{control, status, I2CRegs, I2CRegister};

/// I2C controller device type name
pub const TYPE_I2C_CONTROLLER: &CStr = c"gevico.i2c-rust";

const AT24C02_ADDR: u8 = 0x50;
const AT24C02_SIZE: usize = 256;
const AT24C02_PAGE_SIZE: u8 = 8;

#[derive(Debug)]
struct At24c02 {
    addr: u8,
    storage: [u8; AT24C02_SIZE],
    pointer: u8,
    first_byte: bool,
    page_base: u8,
    page_offset: u8,
}

impl At24c02 {
    fn new(addr: u8) -> Self {
        Self {
            addr,
            storage: [0xFF; AT24C02_SIZE],
            pointer: 0,
            first_byte: true,
            page_base: 0,
            page_offset: 0,
        }
    }
}

impl I2CSlave for At24c02 {
    fn address(&self) -> u8 {
        self.addr
    }

    fn event(&mut self, event: I2CEvent) -> i32 {
        if event == I2CEvent::StartSend {
            self.first_byte = true;
        }
        0
    }

    fn send(&mut self, data: u8) -> i32 {
        if self.first_byte {
            self.pointer = data;
            self.page_base = data & !(AT24C02_PAGE_SIZE - 1);
            self.page_offset = data & (AT24C02_PAGE_SIZE - 1);
            self.first_byte = false;
            return 0;
        }

        let index = self.page_base | self.page_offset;
        self.storage[index as usize] = data;
        self.pointer = index.wrapping_add(1);
        self.page_offset = (self.page_offset + 1) & (AT24C02_PAGE_SIZE - 1);
        0
    }

    fn recv(&mut self) -> u8 {
        let value = self.storage[self.pointer as usize];
        self.pointer = self.pointer.wrapping_add(1);
        value
    }
}

/// I2C controller state
#[derive(Debug)]
pub struct I2CController {
    /// Hardware registers
    pub regs: I2CRegs,
    /// I2C bus managed by this controller
    pub bus: I2CBus,
    /// Current transfer state
    pub transfer_active: bool,
}

impl I2CController {
    /// Create a new I2C controller
    pub fn new() -> Self {
        let mut controller = Self {
            regs: I2CRegs::new(),
            bus: I2CBus::new(),
            transfer_active: false,
        };
        controller.attach_slave(Box::new(At24c02::new(AT24C02_ADDR)));
        controller
    }

    /// Reset the controller to initial state
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Handle MMIO read operation
    pub fn read(&mut self, offset: u32) -> u32 {
        match I2CRegister::from_offset(offset) {
            Some(I2CRegister::Control) => self.regs.control,
            Some(I2CRegister::Status) => self.regs.status,
            Some(I2CRegister::Data) => {
                self.regs.data
            }
            Some(I2CRegister::Address) => self.regs.address,
            Some(I2CRegister::Clock) => self.regs.clock,
            None => {
                // Invalid register offset
                0xFFFFFFFF
            }
        }
    }

    /// Handle MMIO write operation
    pub fn write(&mut self, offset: u32, value: u32) {
        match I2CRegister::from_offset(offset) {
            Some(I2CRegister::Control) => {
                let old_control = self.regs.control;
                self.regs.control = value;
                self.regs.set_done(false);
                self.regs.status &= !status::INT_PEND;

                // Handle enable/disable
                let was_enabled = old_control & control::ENABLE != 0;
                let is_enabled = value & control::ENABLE != 0;

                if !was_enabled && is_enabled {
                    // Controller being enabled
                    self.regs.status |= status::TX_EMPTY;
                } else if was_enabled && !is_enabled {
                    // Controller being disabled
                    if self.transfer_active {
                        self.bus.end_transfer();
                    }
                    self.transfer_active = false;
                    self.regs.set_busy(false);
                }

                // Only process commands if enabled
                if self.regs.is_enabled() {
                    // Handle START condition
                    if value & control::START != 0 {
                        let slave_addr = (self.regs.address & 0x7F) as u8;
                        let is_read = (value & control::READ) != 0;

                        // Start I2C transfer
                        let ret = self.bus.start_transfer(slave_addr, is_read);
                        if ret == 0 {
                            // Slave ACKed
                            self.regs.set_ack(true);
                            self.transfer_active = true;
                            self.regs.set_busy(true);
                        } else {
                            // Slave NACKed
                            self.regs.set_ack(false);
                            self.transfer_active = false;
                        }
                    } else if self.transfer_active && value & control::STOP == 0 {
                        if value & control::READ != 0 {
                            let data = self.bus.recv();
                            self.regs.data = data as u32;
                            self.regs.status |= status::RX_AVAIL;
                            self.regs.set_ack(true);
                        } else {
                            let data = (self.regs.data & 0xFF) as u8;
                            let ret = self.bus.send(data);
                            self.regs.set_ack(ret == 0);
                            if ret != 0 {
                                self.bus.end_transfer();
                                self.transfer_active = false;
                                self.regs.set_busy(false);
                            }
                        }
                    }

                    // Handle STOP condition
                    if value & control::STOP != 0 && self.transfer_active {
                        self.bus.end_transfer();
                        self.transfer_active = false;
                        self.regs.set_busy(false);
                    }
                }

                self.regs.set_done(true);
                if value & control::INT_EN != 0 {
                    self.regs.status |= status::INT_PEND;
                }
            }
            Some(I2CRegister::Status) => {
                // Status register is read-only, but allow clearing some bits
                // Clear interrupt pending flag if written as 1
                if value & status::INT_PEND != 0 {
                    self.regs.status &= !status::INT_PEND;
                }
            }
            Some(I2CRegister::Data) => {
                self.regs.data = value & 0xFF;
                self.regs.status &= !status::RX_AVAIL;
            }
            Some(I2CRegister::Address) => {
                // Only allow address change when not busy
                if !self.regs.is_busy() {
                    self.regs.address = value & 0x7F;
                }
            }
            Some(I2CRegister::Clock) => {
                self.regs.clock = value & 0xFFFF;
            }
            None => {
                // Invalid register offset - ignore
            }
        }
    }

    /// Attach an I2C slave device to the bus
    pub fn attach_slave(&mut self, slave: Box<dyn I2CSlave>) {
        self.bus.attach(slave);
    }

    /// Get the number of slaves attached to the bus
    pub fn slave_count(&self) -> usize {
        self.bus.device_count()
    }
}

impl Default for I2CController {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(test))]
#[repr(C)]
#[derive(qom::Object, hwcore::Device)]
pub struct I2CControllerState {
    pub parent_obj: ParentField<SysBusDevice>,
    pub iomem: MemoryRegion,
    pub irq: InterruptSource,
    pub controller: UnsafeCell<MaybeUninit<I2CController>>,
}

// SAFETY: QEMU invokes this simple MMIO device under the Big QEMU Lock.
#[cfg(not(test))]
unsafe impl Send for I2CControllerState {}
#[cfg(not(test))]
unsafe impl Sync for I2CControllerState {}

#[cfg(not(test))]
impl Default for I2CControllerState {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

#[cfg(not(test))]
qom_isa!(I2CControllerState : SysBusDevice, DeviceState, Object);

#[cfg(not(test))]
#[repr(C)]
pub struct I2CControllerClass {
    parent_class: <SysBusDevice as ObjectType>::Class,
}

#[cfg(not(test))]
trait I2CControllerImpl: SysBusDeviceImpl + IsA<I2CControllerState> {}

#[cfg(not(test))]
impl I2CControllerImpl for I2CControllerState {}

#[cfg(not(test))]
impl I2CControllerClass {
    fn class_init<T: I2CControllerImpl>(&mut self) {
        self.parent_class.class_init::<T>();
    }
}

#[cfg(not(test))]
unsafe impl ObjectType for I2CControllerState {
    type Class = I2CControllerClass;
    const TYPE_NAME: &'static CStr = TYPE_I2C_CONTROLLER;
}

#[cfg(not(test))]
impl I2CControllerState {
    pub unsafe fn init(mut this: ParentInit<I2CControllerState>) {
        static I2C_OPS: MemoryRegionOps<I2CControllerState> =
            MemoryRegionOpsBuilder::new()
                .read(&I2CControllerState::i2c_read)
                .write(&I2CControllerState::i2c_write)
                .little_endian()
                .impl_sizes(4, 4)
                .build();

        MemoryRegion::init_io(
            &mut uninit_field_mut!(*this, iomem),
            &I2C_OPS,
            "gevico.i2c-rust",
            0x1000,
        );
        uninit_field_mut!(*this, irq).write(InterruptSource::default());
        uninit_field_mut!(*this, controller)
            .write(UnsafeCell::new(MaybeUninit::new(I2CController::new())));
    }

    pub fn post_init(&self) {
        self.init_mmio(&self.iomem);
        self.init_irq(&self.irq);
    }

    fn controller_mut(&self) -> &mut I2CController {
        unsafe { (&mut *self.controller.get()).assume_init_mut() }
    }

    fn update_irq(&self) {
        if self.controller_mut().regs.status & status::INT_PEND != 0 {
            self.irq.raise();
        } else {
            self.irq.lower();
        }
    }

    fn i2c_read(&self, offset: u64, _size: u32) -> u64 {
        self.controller_mut().read(offset as u32) as u64
    }

    fn i2c_write(&self, offset: u64, value: u64, _size: u32) {
        self.controller_mut().write(offset as u32, value as u32);
        self.update_irq();
    }

    fn reset_hold(&self, _typ: ResetType) {
        self.controller_mut().reset();
        self.irq.lower();
    }
}

#[cfg(not(test))]
impl ObjectImpl for I2CControllerState {
    type ParentType = SysBusDevice;

    const INSTANCE_INIT: Option<unsafe fn(ParentInit<Self>)> = Some(Self::init);
    const INSTANCE_POST_INIT: Option<fn(&Self)> = Some(Self::post_init);
    const CLASS_INIT: fn(&mut Self::Class) = Self::Class::class_init::<Self>;
}

#[cfg(not(test))]
impl DeviceImpl for I2CControllerState {}

#[cfg(not(test))]
impl ResettablePhasesImpl for I2CControllerState {
    const HOLD: Option<fn(&Self, ResetType)> = Some(Self::reset_hold);
}

#[cfg(not(test))]
impl SysBusDeviceImpl for I2CControllerState {}
