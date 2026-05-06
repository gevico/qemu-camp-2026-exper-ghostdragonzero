// Copyright 2025 HUST OpenAtom Open Source Club.
// SPDX-License-Identifier: GPL-2.0-or-later

//! I2C controller device implementation.

use std::ffi::CStr;

use crate::bus::{I2CBus, I2CSlave};
use crate::register::{control, status, I2CRegs, I2CRegister};

/// I2C controller device type name
pub const TYPE_I2C_CONTROLLER: &CStr = c"i2c-controller";

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
        Self {
            regs: I2CRegs::new(),
            bus: I2CBus::new(),
            transfer_active: false,
        }
    }

    /// Reset the controller to initial state
    pub fn reset(&mut self) {
        self.regs = I2CRegs::new();
        self.transfer_active = false;
    }

    /// Handle MMIO read operation
    pub fn read(&mut self, offset: u32) -> u32 {
        match I2CRegister::from_offset(offset) {
            Some(I2CRegister::Control) => self.regs.control,
            Some(I2CRegister::Status) => self.regs.status,
            Some(I2CRegister::Data) => {
                // Reading from data register triggers I2C read if in read mode
                if self.regs.is_busy() && self.regs.control & control::READ != 0 {
                    let data = self.bus.recv();
                    self.regs.data = data as u32;
                    // Update status: RX data available, TX empty
                    self.regs.status |= status::RX_AVAIL;
                    self.regs.status |= status::TX_EMPTY;
                }
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

                // Handle enable/disable
                let was_enabled = old_control & control::ENABLE != 0;
                let is_enabled = value & control::ENABLE != 0;

                if !was_enabled && is_enabled {
                    // Controller being enabled
                    self.regs.status |= status::TX_EMPTY;
                } else if was_enabled && !is_enabled {
                    // Controller being disabled
                    self.transfer_active = false;
                    self.regs.set_busy(false);
                }

                // Only process commands if enabled
                if self.regs.is_enabled() {
                    // Handle START condition
                    if value & control::START != 0 && !self.transfer_active {
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
                    }

                    // Handle STOP condition
                    if value & control::STOP != 0 && self.transfer_active {
                        self.bus.end_transfer();
                        self.transfer_active = false;
                        self.regs.set_busy(false);
                    }
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
                self.regs.data = value;

                // If in write mode and transfer is active, send data
                if self.regs.is_enabled() && self.transfer_active {
                    if self.regs.control & control::READ == 0 {
                        // Write mode: send byte to slave
                        let data = (value & 0xFF) as u8;
                        let ret = self.bus.send(data);
                        if ret == 0 {
                            self.regs.set_ack(true);
                        } else {
                            self.regs.set_ack(false);
                            // On NACK, end the transfer
                            self.bus.end_transfer();
                            self.transfer_active = false;
                            self.regs.set_busy(false);
                        }
                        // TX buffer becomes empty after write
                        self.regs.status |= status::TX_EMPTY;
                    }
                }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::I2CEvent;

    /// Test slave device for testing
    struct TestSlave {
        addr: u8,
        data: [u8; 256],
        ptr: u8,
    }

    impl TestSlave {
        fn new(addr: u8) -> Self {
            Self {
                addr,
                data: [0xFF; 256],
                ptr: 0,
            }
        }
    }

    impl I2CSlave for TestSlave {
        fn address(&self) -> u8 {
            self.addr
        }

        fn event(&mut self, event: I2CEvent) -> i32 {
            match event {
                I2CEvent::StartSend => {
                    self.ptr = 0;
                    0 // ACK
                }
                I2CEvent::StartRecv => 0, // ACK
                I2CEvent::Finish => 0,
                I2CEvent::Nack => 0,
            }
        }

        fn send(&mut self, data: u8) -> i32 {
            if (self.ptr as usize) < self.data.len() {
                self.data[self.ptr as usize] = data;
                self.ptr = self.ptr.wrapping_add(1);
            }
            0 // ACK
        }

        fn recv(&mut self) -> u8 {
            let val = self.data[self.ptr as usize];
            self.ptr = self.ptr.wrapping_add(1);
            val
        }
    }

    #[test]
    fn test_controller_create() {
        let ctrl = I2CController::new();
        assert!(!ctrl.regs.is_enabled());
        assert!(!ctrl.regs.is_busy());
        assert_eq!(ctrl.slave_count(), 0);
    }

    #[test]
    fn test_controller_enable() {
        let mut ctrl = I2CController::new();
        assert!(!ctrl.regs.is_enabled());

        // Enable controller
        ctrl.write(0x00, control::ENABLE);
        assert!(ctrl.regs.is_enabled());
        assert!(ctrl.regs.status & status::TX_EMPTY != 0);
    }

    #[test]
    fn test_controller_attach_slave() {
        let mut ctrl = I2CController::new();
        assert_eq!(ctrl.slave_count(), 0);

        ctrl.attach_slave(Box::new(TestSlave::new(0x50)));
        assert_eq!(ctrl.slave_count(), 1);

        ctrl.attach_slave(Box::new(TestSlave::new(0x51)));
        assert_eq!(ctrl.slave_count(), 2);
    }

    #[test]
    fn test_controller_i2c_write() {
        let mut ctrl = I2CController::new();
        ctrl.attach_slave(Box::new(TestSlave::new(0x50)));

        // Enable controller
        ctrl.write(0x00, control::ENABLE);

        // Set slave address
        ctrl.write(0x0C, 0x50);

        // Start write transfer
        ctrl.write(0x00, control::ENABLE | control::START);
        assert!(ctrl.transfer_active);
        assert!(ctrl.regs.status & status::ACK != 0);

        // Write data
        ctrl.write(0x08, 0xAB);
        ctrl.write(0x08, 0xCD);

        // Stop transfer
        ctrl.write(0x00, control::ENABLE | control::STOP);
        assert!(!ctrl.transfer_active);
    }

    #[test]
    fn test_controller_i2c_read() {
        let mut ctrl = I2CController::new();
        ctrl.attach_slave(Box::new(TestSlave::new(0x50)));

        // Enable controller
        ctrl.write(0x00, control::ENABLE);

        // Set slave address
        ctrl.write(0x0C, 0x50);

        // Start read transfer
        ctrl.write(0x00, control::ENABLE | control::START | control::READ);
        assert!(ctrl.transfer_active);

        // Read data
        let data1 = ctrl.read(0x08);
        assert_eq!(data1, 0xFF); // Default value from TestSlave

        // Stop transfer
        ctrl.write(0x00, control::ENABLE | control::STOP);
        assert!(!ctrl.transfer_active);
    }

    #[test]
    fn test_controller_invalid_address() {
        let mut ctrl = I2CController::new();
        ctrl.attach_slave(Box::new(TestSlave::new(0x50)));

        // Enable controller
        ctrl.write(0x00, control::ENABLE);

        // Set different slave address
        ctrl.write(0x0C, 0x60);

        // Try to start transfer to non-existent slave
        ctrl.write(0x00, control::ENABLE | control::START);
        assert!(!ctrl.transfer_active); // Should fail
        assert!(ctrl.regs.status & status::ACK == 0); // NACK
    }

    #[test]
    fn test_controller_reset() {
        let mut ctrl = I2CController::new();
        ctrl.attach_slave(Box::new(TestSlave::new(0x50)));

        // Enable and do some operations
        ctrl.write(0x00, control::ENABLE);
        ctrl.write(0x0C, 0x50);
        ctrl.write(0x10, 0x1234);

        // Reset
        ctrl.reset();

        // Check all registers are reset
        assert!(!ctrl.regs.is_enabled());
        assert_eq!(ctrl.regs.address, 0);
        assert_eq!(ctrl.regs.clock, 0);
        assert!(!ctrl.transfer_active);
    }
}
