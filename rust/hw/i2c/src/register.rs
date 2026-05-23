// Copyright 2025 HUST OpenAtom Open Source Club.
// SPDX-License-Identifier: GPL-2.0-or-later

//! I2C controller register definitions.

/// I2C controller register offsets
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum I2CRegister {
    /// Control register
    Control = 0x00,
    /// Status register
    Status = 0x04,
    /// Slave address register
    Address = 0x08,
    /// Data register
    Data = 0x0C,
    /// Clock divider register
    Clock = 0x10,
}

impl I2CRegister {
    /// Convert from offset value
    pub fn from_offset(offset: u32) -> Option<Self> {
        match offset {
            0x00 => Some(Self::Control),
            0x04 => Some(Self::Status),
            0x08 => Some(Self::Address),
            0x0C => Some(Self::Data),
            0x10 => Some(Self::Clock),
            _ => None,
        }
    }
}

/// Control register bits
pub mod control {
    /// Enable I2C controller
    pub const ENABLE: u32 = 0x0001;
    /// Send START condition
    pub const START: u32 = 0x0002;
    /// Send STOP condition
    pub const STOP: u32 = 0x0004;
    /// Read direction (when set) / Write direction (when clear)
    pub const READ: u32 = 0x0008;
    /// Interrupt enable
    pub const INT_EN: u32 = 0x0010;
}

/// Status register bits
pub mod status {
    /// Controller is busy
    pub const BUSY: u32 = 0x0001;
    /// ACK received from slave
    pub const ACK: u32 = 0x0002;
    /// Current command completed
    pub const DONE: u32 = 0x0004;
    /// Data available in receive buffer
    pub const RX_AVAIL: u32 = 0x0008;
    /// Transmit buffer empty
    pub const TX_EMPTY: u32 = 0x0010;
    /// Interrupt pending
    pub const INT_PEND: u32 = 0x0020;
    /// Arbitration lost
    pub const ARB_LOST: u32 = 0x0040;
}

/// I2C controller register state
#[derive(Debug, Clone, Copy, Default)]
pub struct I2CRegs {
    /// Control register value
    pub control: u32,
    /// Status register value
    pub status: u32,
    /// Data register value
    pub data: u32,
    /// Slave address register value
    pub address: u32,
    /// Clock divider register value
    pub clock: u32,
}

impl I2CRegs {
    /// Create new register set with default values
    pub fn new() -> Self {
        Self {
            control: 0,
            status: 0,
            data: 0,
            address: 0,
            clock: 0,
        }
    }

    /// Check if controller is enabled
    pub fn is_enabled(&self) -> bool {
        self.control & control::ENABLE != 0
    }

    /// Check if controller is busy
    pub fn is_busy(&self) -> bool {
        self.status & status::BUSY != 0
    }

    /// Set busy flag
    pub fn set_busy(&mut self, busy: bool) {
        if busy {
            self.status |= status::BUSY;
        } else {
            self.status &= !status::BUSY;
        }
    }

    /// Set ACK flag
    pub fn set_ack(&mut self, ack: bool) {
        if ack {
            self.status |= status::ACK;
        } else {
            self.status &= !status::ACK;
        }
    }

    /// Set DONE flag
    pub fn set_done(&mut self, done: bool) {
        if done {
            self.status |= status::DONE;
        } else {
            self.status &= !status::DONE;
        }
    }
}
