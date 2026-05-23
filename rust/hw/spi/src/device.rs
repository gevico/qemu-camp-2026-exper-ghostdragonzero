// Copyright 2026 HUST OpenAtom Open Source Club.
// SPDX-License-Identifier: GPL-2.0-or-later

use std::{cell::UnsafeCell, ffi::CStr, mem::MaybeUninit, ptr};

use common::prelude::*;
use hwcore::prelude::*;
use qom::prelude::*;
use system::prelude::*;

use crate::bindings;
use crate::bus::SSIBus;
use crate::peripheral::TYPE_AT25_RUST;
use crate::registers::{cr1, offset, sr, RSPI_SIZE};

pub const TYPE_RUST_SPI: &CStr = c"gevico.spi-rust";

#[derive(Debug)]
struct RustSpi {
    cr1: u32,
    sr: u32,
    dr: u32,
    cs: u32,
    bus: *mut SSIBus,
}

impl RustSpi {
    fn new(bus: *mut SSIBus) -> Self {
        Self {
            cr1: 0,
            sr: 0,
            dr: 0,
            cs: 0,
            bus,
        }
    }

    fn reset(&mut self) {
        let bus = self.bus;
        *self = Self::new(bus);
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
                let rx = if self.cs == 0 && !self.bus.is_null() {
                    self.bus_ref().transfer(self.dr) & 0xFF
                } else {
                    self.dr
                };
                self.dr = rx;
                self.sr |= sr::RXNE | sr::TXE;
            }
            offset::CS => {
                self.cs = value & 0x3;
            }
            _ => {}
        }
    }

    fn bus_ref(&self) -> &SSIBus {
        unsafe { &*self.bus }
    }
}

#[repr(C)]
#[derive(qom::Object, hwcore::Device)]
pub struct RustSpiState {
    pub parent_obj: ParentField<SysBusDevice>,
    pub iomem: MemoryRegion,
    pub irq: InterruptSource,
    pub bus: *mut SSIBus,
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
        let dev: &mut DeviceState = unsafe { this.upcast_mut() };
        let bus = unsafe { SSIBus::create(hwcore::DeviceState::as_mut_ptr(dev), c"ssi") };
        let flash = unsafe { hwcore::bindings::qdev_new(TYPE_AT25_RUST.as_ptr()) };
        unsafe {
            bindings::ssi_realize_and_unref(
                flash,
                bus.cast(),
                ptr::addr_of_mut!(util::bindings::error_abort),
            );
        }
        uninit_field_mut!(*this, bus).write(bus);
        uninit_field_mut!(*this, state)
            .write(UnsafeCell::new(MaybeUninit::new(RustSpi::new(bus))));
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
