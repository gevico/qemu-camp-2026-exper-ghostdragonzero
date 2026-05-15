// Copyright 2024, QEMU Camp 2026
// SPDX-License-Identifier: GPL-2.0-or-later

//! SiFive GPIO device registers
//!
//! Simple GPIO implementation for learning purposes

use bilge::prelude::*;

/// Number of GPIO pins supported by this device
pub const SIFIVE_GPIO_PINS: u32 = 32;

/// Size of the GPIO register space
pub const SIFIVE_GPIO_SIZE: u64 = 0x100;

/// Offset of each register from the base memory address of the device.
#[allow(non_camel_case_types)]
#[repr(u64)]
#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum RegisterOffset {
    /// Pin Value Register
    VALUE = 0x000,
    /// Input Enable Register
    INPUT_EN = 0x004,
    /// Output Enable Register
    OUTPUT_EN = 0x008,
    /// Port Register
    PORT = 0x00C,
    /// Internal Pull-Up Enable Register
    PUE = 0x010,
    /// Drive Strength Register
    DS = 0x014,
    /// Rise Interrupt Enable Register
    RISE_IE = 0x018,
    /// Rise Interrupt Pending Register
    RISE_IP = 0x01C,
    /// Fall Interrupt Enable Register
    FALL_IE = 0x020,
    /// Fall Interrupt Pending Register
    FALL_IP = 0x024,
    /// High Interrupt Enable Register
    HIGH_IE = 0x028,
    /// High Interrupt Pending Register
    HIGH_IP = 0x02C,
    /// Low Interrupt Enable Register
    LOW_IE = 0x030,
    /// Low Interrupt Pending Register
    LOW_IP = 0x034,
    /// IO Function Enable Register
    IOF_EN = 0x038,
    /// IO Function Select Register
    IOF_SEL = 0x03C,
    /// Output XOR Register
    OUT_XOR = 0x040,
}

/// Simple GPIO register - 32 bits, one bit per pin
///
/// Most GPIO registers use a simple 1-bit-per-pin layout where:
/// - Bit n corresponds to GPIO pin n
/// - Reading returns the current state/values
/// - Writing sets the values (for output registers)
#[bitsize(32)]
#[derive(Clone, Copy, Default, DebugBits, FromBits)]
pub struct GpioReg {
    /// GPIO pin values (bit 0 = pin 0, bit 1 = pin 1, etc.)
    pub pins: u32,
}

impl GpioReg {
    /// Get the value of a specific pin
    pub fn get_pin(&self, pin: u32) -> bool {
        debug_assert!(pin < SIFIVE_GPIO_PINS);
        (self.pins() >> pin) & 1 == 1
    }

    /// Set the value of a specific pin
    pub fn set_pin(&mut self, pin: u32, value: bool) {
        debug_assert!(pin < SIFIVE_GPIO_PINS);
        let mask = 1 << pin;
        let mut pins = self.pins();
        if value {
            pins |= mask;
        } else {
            pins &= !mask;
        }
        self.set_pins(pins);
    }
}

/// Pin Value Register
///
/// Contains the current value of all GPIO pins
#[allow(dead_code)]
pub type Value = GpioReg;

/// Input Enable Register
///
/// Controls which pins are configured as inputs
#[allow(dead_code)]
pub type InputEn = GpioReg;

/// Output Enable Register
///
/// Controls which pins are configured as outputs
#[allow(dead_code)]
pub type OutputEn = GpioReg;

/// Port Register
///
/// Used for atomic read-modify-write operations on output pins
#[allow(dead_code)]
pub type Port = GpioReg;

/// Pull-Up Enable Register
///
/// Enables internal pull-up resistors for each pin
#[allow(dead_code)]
pub type PullUpEn = GpioReg;

/// Drive Strength Register
///
/// Controls drive strength for each pin
#[allow(dead_code)]
pub type DriveStrength = GpioReg;

/// Interrupt Enable Registers (Rise, Fall, High, Low)
///
/// Controls which pins can generate interrupts
#[allow(dead_code)]
pub type RiseIE = GpioReg;
#[allow(dead_code)]
pub type RiseIP = GpioReg;
#[allow(dead_code)]
pub type FallIE = GpioReg;
#[allow(dead_code)]
pub type FallIP = GpioReg;
#[allow(dead_code)]
pub type HighIE = GpioReg;
#[allow(dead_code)]
pub type HighIP = GpioReg;
#[allow(dead_code)]
pub type LowIE = GpioReg;
#[allow(dead_code)]
pub type LowIP = GpioReg;

/// IO Function Enable Register
///
/// Enables hardware IO functions (PWM, UART, etc.) on pins
#[allow(dead_code)]
pub type IofEn = GpioReg;

/// IO Function Select Register
///
/// Selects which hardware function is assigned to each pin
#[allow(dead_code)]
pub type IofSel = GpioReg;

/// Output XOR Register
///
/// Inverts output values for selected pins
#[allow(dead_code)]
pub type OutXor = GpioReg;
