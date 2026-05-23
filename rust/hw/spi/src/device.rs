// Copyright 2026 HUST OpenAtom Open Source Club.
// SPDX-License-Identifier: GPL-2.0-or-later

use std::{cell::UnsafeCell, ffi::CStr, mem::MaybeUninit};

use common::prelude::*;
use hwcore::prelude::*;
use qom::prelude::*;
use system::prelude::*;

use crate::registers::{cr1, offset, sr, RSPI_SIZE};

pub const TYPE_RUST_SPI: &CStr = c"gevico.spi-rust";

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
struct At25 {
    storage: [u8; AT25_SIZE],
    status: u8,
    mode: FlashMode,
    pointer: u8,
    write_count: u8,
}

impl At25 {
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

#[derive(Debug)]
struct RustSpi {
    cr1: u32,
    sr: u32,
    dr: u32,
    cs: u32,
    flash: At25,
}

impl RustSpi {
    fn new() -> Self {
        Self {
            cr1: 0,
            sr: 0,
            dr: 0,
            cs: 0,
            flash: At25::new(),
        }
    }

    fn reset(&mut self) {
        *self = Self::new();
    }

    fn read(&mut self, reg: u32) -> u32 {
        match reg {
            offset::CR1 => self.cr1,
            offset::SR => self.sr,
            offset::DR => {
                let value = self.dr;
                self.sr &= !sr::RXNE;
                value
            }
            offset::CS => self.cs,
            _ => 0,
        }
    }

    fn write(&mut self, reg: u32, value: u32) {
        match reg {
            offset::CR1 => {
                self.cr1 = value & (cr1::SPE | cr1::MSTR);
                if self.cr1 & cr1::SPE != 0 {
                    self.sr |= sr::TXE;
                } else {
                    self.sr = 0;
                }
            }
            offset::SR => {
                if value & sr::OVERRUN != 0 {
                    self.sr &= !sr::OVERRUN;
                }
            }
            offset::DR => {
                self.dr = value & 0xFF;
                if self.cr1 & cr1::SPE == 0 || self.cr1 & cr1::MSTR == 0 {
                    return;
                }
                let rx = if self.cs == 0 {
                    self.flash.transfer(self.dr as u8)
                } else {
                    self.dr as u8
                };
                self.dr = rx as u32;
                self.sr |= sr::RXNE | sr::TXE;
            }
            offset::CS => {
                self.cs = value & 0x3;
            }
            _ => {}
        }
    }
}

#[repr(C)]
#[derive(qom::Object, hwcore::Device)]
pub struct RustSpiState {
    pub parent_obj: ParentField<SysBusDevice>,
    pub iomem: MemoryRegion,
    pub irq: InterruptSource,
    state: UnsafeCell<MaybeUninit<RustSpi>>,
}

// SAFETY: QEMU invokes this simple MMIO device under the Big QEMU Lock.
unsafe impl Send for RustSpiState {}
unsafe impl Sync for RustSpiState {}

impl Default for RustSpiState {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

qom_isa!(RustSpiState : SysBusDevice, DeviceState, Object);

#[repr(C)]
pub struct RustSpiClass {
    parent_class: <SysBusDevice as ObjectType>::Class,
}

trait RustSpiImpl: SysBusDeviceImpl + IsA<RustSpiState> {}

impl RustSpiImpl for RustSpiState {}

impl RustSpiClass {
    fn class_init<T: RustSpiImpl>(&mut self) {
        self.parent_class.class_init::<T>();
    }
}

unsafe impl ObjectType for RustSpiState {
    type Class = RustSpiClass;
    const TYPE_NAME: &'static CStr = TYPE_RUST_SPI;
}

impl RustSpiState {
    pub unsafe fn init(mut this: ParentInit<RustSpiState>) {
        static RSPI_OPS: MemoryRegionOps<RustSpiState> =
            MemoryRegionOpsBuilder::new()
                .read(&RustSpiState::mmio_read)
                .write(&RustSpiState::mmio_write)
                .little_endian()
                .impl_sizes(4, 4)
                .build();

        MemoryRegion::init_io(
            &mut uninit_field_mut!(*this, iomem),
            &RSPI_OPS,
            "gevico.spi-rust",
            RSPI_SIZE,
        );
        uninit_field_mut!(*this, irq).write(InterruptSource::default());
        uninit_field_mut!(*this, state).write(UnsafeCell::new(MaybeUninit::new(RustSpi::new())));
    }

    pub fn post_init(&self) {
        self.init_mmio(&self.iomem);
        self.init_irq(&self.irq);
    }

    fn state_mut(&self) -> &mut RustSpi {
        unsafe { (&mut *self.state.get()).assume_init_mut() }
    }

    fn mmio_read(&self, offset: u64, _size: u32) -> u64 {
        self.state_mut().read(offset as u32) as u64
    }

    fn mmio_write(&self, offset: u64, value: u64, _size: u32) {
        self.state_mut().write(offset as u32, value as u32);
    }

    fn reset_hold(&self, _typ: ResetType) {
        self.state_mut().reset();
        self.irq.lower();
    }
}

impl ObjectImpl for RustSpiState {
    type ParentType = SysBusDevice;

    const INSTANCE_INIT: Option<unsafe fn(ParentInit<Self>)> = Some(Self::init);
    const INSTANCE_POST_INIT: Option<fn(&Self)> = Some(Self::post_init);
    const CLASS_INIT: fn(&mut Self::Class) = Self::Class::class_init::<Self>;
}

impl DeviceImpl for RustSpiState {}

impl ResettablePhasesImpl for RustSpiState {
    const HOLD: Option<fn(&Self, ResetType)> = Some(Self::reset_hold);
}

impl SysBusDeviceImpl for RustSpiState {}
