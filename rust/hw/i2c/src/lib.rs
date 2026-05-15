// Copyright 2025 HUST OpenAtom Open Source Club.
// Author(s): Chen Miao <chenmiao@openatom.club>
// Author(s): Chao Liu <chao.liu@openatom.club>
// SPDX-License-Identifier: GPL-2.0-or-later

//! I2C bus model for G233 SoC (QEMU Camp 2026 Rust experiment).
//!
//! This module provides a pure-Rust I2C bus abstraction modeled after
//! the upstream QEMU C I2C infrastructure (`include/hw/i2c/i2c.h`).
//!
//! Students must implement the TODO-marked methods to make the unit
//! tests pass. The design follows the upstream pattern:
//!
//! - [`I2CEvent`]: bus state change events (START_RECV, START_SEND, FINISH, NACK)
//! - [`I2CSlave`]: trait for I2C slave devices (send, recv, event)
//! - [`I2CBus`]: bus that manages slave devices and routes transfers

pub mod bindings;
pub mod bus;
pub mod device;
pub mod register;

pub use bus::{I2CBus, I2CSlave, I2CEvent};
pub use device::{I2CController, TYPE_I2C_CONTROLLER};
pub use register::{I2CRegs, I2CRegister, control, status};

// ─── Unit tests ──────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    /// Mock EEPROM device: 256-byte register file, supports I2C protocol.
    /// Write: first byte = register address, subsequent bytes = data.
    /// Read: returns bytes starting from the last set register address.
    struct MockEeprom {
        addr: u8,
        regs: [u8; 256],
        pointer: u8,
        first_byte: bool,
    }

    impl MockEeprom {
        fn new(addr: u8) -> Self {
            Self {
                addr,
                regs: [0xFF; 256],
                pointer: 0,
                first_byte: true,
            }
        }
    }

    impl I2CSlave for MockEeprom {
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
                self.first_byte = false;
            } else {
                self.regs[self.pointer as usize] = data;
                self.pointer = self.pointer.wrapping_add(1);
            }
            0
        }

        fn recv(&mut self) -> u8 {
            let val = self.regs[self.pointer as usize];
            self.pointer = self.pointer.wrapping_add(1);
            val
        }
    }

    #[test]
    fn test_i2c_bus_create() {
        let mut bus = I2CBus::new();
        bus.attach(Box::new(MockEeprom::new(0x50)));
        assert_eq!(bus.device_count(), 1, "bus should have 1 device");
        bus.attach(Box::new(MockEeprom::new(0x51)));
        assert_eq!(bus.device_count(), 2, "bus should have 2 devices");
    }

    #[test]
    fn test_i2c_bus_read_write() {
        let mut bus = I2CBus::new();
        bus.attach(Box::new(MockEeprom::new(0x50)));

        // Write: addr_byte=0x10, data=0xDE
        let ack = bus.transfer_write(0x50, &[0x10, 0xDE]);
        assert!(ack, "write to attached device must ACK");

        // Set read pointer: write register address first
        bus.transfer_write(0x50, &[0x10]);

        // Read back 1 byte
        let data = bus.transfer_read(0x50, 1);
        assert_eq!(data, Some(vec![0xDE]), "read back must match written value");
    }

    #[test]
    fn test_i2c_bus_nack() {
        let mut bus = I2CBus::new();
        bus.attach(Box::new(MockEeprom::new(0x50)));

        // Read from non-existent address
        let data = bus.transfer_read(0x77, 1);
        assert_eq!(data, None, "read from non-existent address must NACK");

        // Write to non-existent address
        let ack = bus.transfer_write(0x77, &[0x00]);
        assert!(!ack, "write to non-existent address must NACK");
    }
}
