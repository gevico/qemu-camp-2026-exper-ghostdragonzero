// Copyright 2024, QEMU Camp 2026
// SPDX-License-Identifier: GPL-2.0-or-later

//! SiFive GPIO device model implementation
//!
//! Simplified version for learning purposes - implements QEMU device interface

use std::mem::size_of;
use std::cell::UnsafeCell;

use common::prelude::*;
use hwcore::prelude::*;
use qom::prelude::*;
use system::prelude::*;

use crate::registers::{RegisterOffset, SIFIVE_GPIO_PINS, SIFIVE_GPIO_SIZE};

/// SiFive GPIO device state
#[repr(C)]
#[derive(qom::Object, hwcore::Device)]
pub struct SiFiveGpioState {
    pub parent_obj: ParentField<SysBusDevice>,
    pub iomem: MemoryRegion,

    // GPIO IRQ output lines - one per pin (matches C struct)
    pub irq: [InterruptSource; 32],
    // GPIO output lines - one per pin (matches C struct)
    pub output: [InterruptSource; 32],

    // GPIO registers - wrapped in UnsafeCell for interior mutability
    pub value: UnsafeCell<u32>,
    pub input_en: UnsafeCell<u32>,
    pub output_en: UnsafeCell<u32>,
    pub port: UnsafeCell<u32>,
    pub pue: UnsafeCell<u32>,
    pub ds: UnsafeCell<u32>,
    pub rise_ie: UnsafeCell<u32>,
    pub rise_ip: UnsafeCell<u32>,
    pub fall_ie: UnsafeCell<u32>,
    pub fall_ip: UnsafeCell<u32>,
    pub high_ie: UnsafeCell<u32>,
    pub high_ip: UnsafeCell<u32>,
    pub low_ie: UnsafeCell<u32>,
    pub low_ip: UnsafeCell<u32>,
    pub iof_en: UnsafeCell<u32>,
    pub iof_sel: UnsafeCell<u32>,
    pub out_xor: UnsafeCell<u32>,
    // Input pin values (matches C struct)
    #[doc(alias = "in")]
    pub in_: UnsafeCell<u32>,
    pub in_mask: UnsafeCell<u32>,
    pub ngpio: u32,
}

// SAFETY: The device is only accessed from a single thread at a time due to BQL
unsafe impl Send for SiFiveGpioState {}
unsafe impl Sync for SiFiveGpioState {}

impl Default for SiFiveGpioState {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

// Ensure the Rust state is not larger than the C state
static_assert!(size_of::<SiFiveGpioState>() <= size_of::<crate::bindings::SIFIVEGPIOState>());

qom_isa!(SiFiveGpioState : SysBusDevice, DeviceState, Object);

#[repr(C)]
pub struct SiFiveGpioClass {
    parent_class: <SysBusDevice as ObjectType>::Class,
}

// Trait for GPIO device implementation
trait SiFiveGpioImpl: SysBusDeviceImpl + IsA<SiFiveGpioState> {}

impl SiFiveGpioImpl for SiFiveGpioState {}

impl SiFiveGpioClass {
    fn class_init<T: SiFiveGpioImpl>(&mut self) {
        self.parent_class.class_init::<T>();
    }
}

unsafe impl ObjectType for SiFiveGpioState {
    type Class = SiFiveGpioClass;
    const TYPE_NAME: &'static ::std::ffi::CStr = crate::TYPE_SIFIVE_GPIO;
}

impl SiFiveGpioState {
    /// Initialize GPIO device - called during device creation
    pub unsafe fn init(mut this: ParentInit<SiFiveGpioState>) {
        static GPIO_OPS: MemoryRegionOps<SiFiveGpioState> =
            MemoryRegionOpsBuilder::new()
                .read(&SiFiveGpioState::gpio_read)
                .write(&SiFiveGpioState::gpio_write)
                .little_endian()
                .impl_sizes(4, 4)
                .build();

        // Initialize MMIO region
        MemoryRegion::init_io(
            &mut uninit_field_mut!(*this, iomem),
            &GPIO_OPS,
            "sifive.gpio",
            SIFIVE_GPIO_SIZE,
        );

        // Initialize registers to 0
        uninit_field_mut!(*this, value).write(UnsafeCell::new(0));
        uninit_field_mut!(*this, input_en).write(UnsafeCell::new(0));
        uninit_field_mut!(*this, output_en).write(UnsafeCell::new(0));
        uninit_field_mut!(*this, port).write(UnsafeCell::new(0));
        uninit_field_mut!(*this, pue).write(UnsafeCell::new(0));
        uninit_field_mut!(*this, ds).write(UnsafeCell::new(0));
        uninit_field_mut!(*this, rise_ie).write(UnsafeCell::new(0));
        uninit_field_mut!(*this, rise_ip).write(UnsafeCell::new(0));
        uninit_field_mut!(*this, fall_ie).write(UnsafeCell::new(0));
        uninit_field_mut!(*this, fall_ip).write(UnsafeCell::new(0));
        uninit_field_mut!(*this, high_ie).write(UnsafeCell::new(0));
        uninit_field_mut!(*this, high_ip).write(UnsafeCell::new(0));
        uninit_field_mut!(*this, low_ie).write(UnsafeCell::new(0));
        uninit_field_mut!(*this, low_ip).write(UnsafeCell::new(0));
        uninit_field_mut!(*this, iof_en).write(UnsafeCell::new(0));
        uninit_field_mut!(*this, iof_sel).write(UnsafeCell::new(0));
        uninit_field_mut!(*this, out_xor).write(UnsafeCell::new(0));
        uninit_field_mut!(*this, ngpio).write(SIFIVE_GPIO_PINS);
    }

    /// Post-initialization - setup MMIO and IRQs
    pub fn post_init(&self) {
        // Initialize MMIO region
        self.init_mmio(&self.iomem);

        // Initialize SysBusDevice IRQ inputs (32 pins, one IRQ input per pin)
        for i in 0..32 {
            if i < self.ngpio as usize {
                self.init_irq(&self.irq[i]);
            }
        }
    }

    /// Read from GPIO register
    fn gpio_read(&self, offset: u64, _size: u32) -> u64 {
        let value = match offset as u32 {
            x if x == RegisterOffset::VALUE as u32 => unsafe { *self.value.get() },
            x if x == RegisterOffset::INPUT_EN as u32 => unsafe { *self.input_en.get() },
            x if x == RegisterOffset::OUTPUT_EN as u32 => unsafe { *self.output_en.get() },
            x if x == RegisterOffset::PORT as u32 => unsafe { *self.port.get() },
            x if x == RegisterOffset::PUE as u32 => unsafe { *self.pue.get() },
            x if x == RegisterOffset::DS as u32 => unsafe { *self.ds.get() },
            x if x == RegisterOffset::RISE_IE as u32 => unsafe { *self.rise_ie.get() },
            x if x == RegisterOffset::RISE_IP as u32 => unsafe { *self.rise_ip.get() },
            x if x == RegisterOffset::FALL_IE as u32 => unsafe { *self.fall_ie.get() },
            x if x == RegisterOffset::FALL_IP as u32 => unsafe { *self.fall_ip.get() },
            x if x == RegisterOffset::HIGH_IE as u32 => unsafe { *self.high_ie.get() },
            x if x == RegisterOffset::HIGH_IP as u32 => unsafe { *self.high_ip.get() },
            x if x == RegisterOffset::LOW_IE as u32 => unsafe { *self.low_ie.get() },
            x if x == RegisterOffset::LOW_IP as u32 => unsafe { *self.low_ip.get() },
            x if x == RegisterOffset::IOF_EN as u32 => unsafe { *self.iof_en.get() },
            x if x == RegisterOffset::IOF_SEL as u32 => unsafe { *self.iof_sel.get() },
            x if x == RegisterOffset::OUT_XOR as u32 => unsafe { *self.out_xor.get() },
            _ => {
                eprintln!("{}: bad read offset 0x{:x}", "SiFive GPIO", offset);
                0
            }
        };

        eprintln!("GPIO read offset 0x{:x}, value 0x{:x}", offset, value);
        value as u64
    }

    /// Write to GPIO register
    fn gpio_write(&self, offset: u64, value: u64, _size: u32) {
        eprintln!("GPIO write offset 0x{:x}, value 0x{:x}", offset, value);

        let value = value as u32;

        match offset as u32 {
            x if x == RegisterOffset::INPUT_EN as u32 => {
                unsafe { *self.input_en.get() = value };
            }
            x if x == RegisterOffset::OUTPUT_EN as u32 => {
                unsafe { *self.output_en.get() = value };
            }
            x if x == RegisterOffset::PORT as u32 => {
                unsafe { *self.port.get() = value };
            }
            x if x == RegisterOffset::PUE as u32 => {
                unsafe { *self.pue.get() = value };
            }
            x if x == RegisterOffset::DS as u32 => {
                unsafe { *self.ds.get() = value };
            }
            x if x == RegisterOffset::RISE_IE as u32 => {
                unsafe { *self.rise_ie.get() = value };
            }
            x if x == RegisterOffset::RISE_IP as u32 => {
                unsafe { *self.rise_ip.get() &= !value };
            }
            x if x == RegisterOffset::FALL_IE as u32 => {
                unsafe { *self.fall_ie.get() = value };
            }
            x if x == RegisterOffset::FALL_IP as u32 => {
                unsafe { *self.fall_ip.get() &= !value };
            }
            x if x == RegisterOffset::HIGH_IE as u32 => {
                unsafe { *self.high_ie.get() = value };
            }
            x if x == RegisterOffset::HIGH_IP as u32 => {
                unsafe { *self.high_ip.get() &= !value };
            }
            x if x == RegisterOffset::LOW_IE as u32 => {
                unsafe { *self.low_ie.get() = value };
            }
            x if x == RegisterOffset::LOW_IP as u32 => {
                unsafe { *self.low_ip.get() &= !value };
            }
            x if x == RegisterOffset::IOF_EN as u32 => {
                unsafe { *self.iof_en.get() = value };
            }
            x if x == RegisterOffset::IOF_SEL as u32 => {
                unsafe { *self.iof_sel.get() = value };
            }
            x if x == RegisterOffset::OUT_XOR as u32 => {
                unsafe { *self.out_xor.get() = value };
            }
            _ => {
                eprintln!("{}: bad write offset 0x{:x}", "SiFive GPIO", offset);
            }
        }
    }
}

// Trait implementations for QEMU device integration
impl SysBusDeviceImpl for SiFiveGpioState {}

impl ObjectImpl for SiFiveGpioState {
    type ParentType = SysBusDevice;

    const INSTANCE_INIT: Option<unsafe fn(ParentInit<Self>)> = Some(Self::init);
    const INSTANCE_POST_INIT: Option<fn(&Self)> = Some(Self::post_init);
    const CLASS_INIT: fn(&mut Self::Class) = Self::Class::class_init::<Self>;
}

impl DeviceImpl for SiFiveGpioState {
    // Minimal device implementation
}

impl ResettablePhasesImpl for SiFiveGpioState {
    // No reset implementation for now
}
