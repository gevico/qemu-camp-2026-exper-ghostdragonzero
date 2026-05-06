// SPDX-License-Identifier: GPL-2.0-or-later

//pub use crate::{I2CBus, I2CEvent, I2CSlave};
// ─── I2C Event ───────────────────────────────────────────────────────

/// I2C bus events, mirroring `enum i2c_event` from upstream C code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum I2CEvent {
    /// Master requests to read from slave
    StartRecv,
    /// Master requests to write to slave
    StartSend,
    /// Transfer finished
    Finish,
    /// Master NACKed a received byte
    Nack,
}

// ─── I2C Slave trait ─────────────────────────────────────────────────

/// Trait representing an I2C slave device on the bus.
///
/// This mirrors `I2CSlaveClass` from upstream QEMU:
/// - `address()`: the 7-bit slave address
/// - `event()`: notification of bus state changes (START/FINISH/NACK)
/// - `send()`: master-to-slave data byte, returns 0 for ACK, non-zero for NACK
/// - `recv()`: slave-to-master data byte
pub trait I2CSlave {
    /// Return the 7-bit I2C address of this device.
    fn address(&self) -> u8;

    /// Notify the slave of a bus state change.
    ///
    /// For `StartRecv`/`StartSend`, return 0 to ACK or non-zero to NACK.
    /// For `Finish`/`Nack`, the return value is ignored.
    fn event(&mut self, event: I2CEvent) -> i32 {
        let _ = event;
        0
    }

    /// Master sends a data byte to this slave.
    /// Returns 0 for ACK, non-zero for NACK.
    fn send(&mut self, data: u8) -> i32;

    /// Slave returns a data byte to the master.
    fn recv(&mut self) -> u8;
}

// ─── I2C Bus ─────────────────────────────────────────────────────────

/// A simple I2C bus that manages a list of slave devices.
///
/// Mirrors `I2CBus` from upstream QEMU. The bus holds references to
/// attached slaves and routes transfers based on 7-bit address matching.
#[allow(dead_code)]
pub struct I2CBus {
    devices: Vec<Box<dyn I2CSlave>>,
    /// Address of the currently selected slave (set by `start_transfer`)
    current_addr: Option<u8>,
    /// Transfer direction: true = recv (slave→master), false = send (master→slave)
    is_recv: bool,
}

impl std::fmt::Debug for I2CBus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("I2CBus")
            .field("device_count", &self.devices.len())
            .field("current_addr", &self.current_addr)
            .field("is_recv", &self.is_recv)
            .finish()
    }
}

impl I2CBus {
    /// Create an empty bus with no attached devices.
    pub fn new() -> Self {
        Self {
            devices: Vec::new(),
            current_addr: None,
            is_recv: false,
        }
    }

    /// Attach a slave device to the bus.
    pub fn attach(&mut self, device: Box<dyn I2CSlave>) {
        // TODO: push the device onto the bus
        self.devices.push(device);
    }

    /// Return the number of devices on the bus.
    pub fn device_count(&self) -> usize {
        // TODO: return actual count
        self.devices.len()
    }

    /// Check if the bus is busy (a transfer is in progress).
    pub fn is_busy(&self) -> bool {
        self.current_addr.is_some()
    }

    /// Start a transfer to the slave at `address`.
    ///
    /// If `is_recv` is true, the master wants to read (START_RECV).
    /// If `is_recv` is false, the master wants to write (START_SEND).
    ///
    /// Returns 0 on success (slave ACKed), -1 if no slave responds (NACK).
    ///
    /// This mirrors `i2c_start_transfer()` from upstream.
    pub fn start_transfer(&mut self, address: u8, is_recv: bool) -> i32 {
        // TODO: find a device matching _address, call its event()
        // with StartRecv or StartSend. If ACKed, store current_addr
        // and is_recv. Return 0 on ACK, -1 on NACK.
        for device in &mut self.devices {
          if device.address() == address {
              // 确定事件类型
              let event = if is_recv {
                  I2CEvent::StartRecv
              } else {
                  I2CEvent::StartSend
              };

              // 调用设备的 event 方法
              let ret = device.event(event);

              // 如果设备 ACK（返回 0），保存当前传输状态
              if ret == 0 {
                  self.current_addr = Some(address);
                  self.is_recv = is_recv;
                  return 0; // ACK
              }
          }
      }
        -1
    }

    /// End the current transfer, sending Finish event to the active slave.
    ///
    /// Mirrors `i2c_end_transfer()` from upstream.
    pub fn end_transfer(&mut self) {
        // TODO: send Finish event to the current slave, clear current_addr
        if let Some(addr) = self.current_addr {
          // 找到当前活跃的设备并发送 Finish 事件
          for device in &mut self.devices {
              if device.address() == addr {
                  device.event(I2CEvent::Finish);
                  break;
              }
          }
          // 清除当前地址
          self.current_addr = None;
      }
    }

    /// Send a data byte from master to the current slave.
    ///
    /// Returns 0 for ACK, non-zero for NACK.
    /// Mirrors `i2c_send()` from upstream.
    pub fn send(&mut self, data: u8) -> i32 {
        // TODO: call send() on the current slave
      if let Some(addr) = self.current_addr {
          for device in &mut self.devices {
              if device.address() == addr {
                  return device.send(data);
              }
          }
      }
      -1
    }

    /// Receive a data byte from the current slave to master.
    ///
    /// Mirrors `i2c_recv()` from upstream.
    pub fn recv(&mut self) -> u8 {
        // TODO: call recv() on the current slave
        if let Some(addr) = self.current_addr {
          for device in &mut self.devices {
              if device.address() == addr {
                  return device.recv();
              }
          }
        }
        0xFF
    }

    // ── Convenience helpers (used by unit tests) ──

    /// Perform a complete write transfer: START_SEND + send bytes + FINISH.
    ///
    /// Returns true on ACK, false on NACK.
    pub fn transfer_write(&mut self, addr: u8, data: &[u8]) -> bool {
        if self.start_transfer(addr, false) != 0 {
            return false;
        }
        for &byte in data {
            if self.send(byte) != 0 {
                self.end_transfer();
                return false;
            }
        }
        self.end_transfer();
        true
    }

    /// Perform a complete read transfer: START_RECV + recv `len` bytes + FINISH.
    ///
    /// Returns None on NACK, Some(bytes) on success.
    pub fn transfer_read(&mut self, addr: u8, len: usize) -> Option<Vec<u8>> {
        if self.start_transfer(addr, true) != 0 {
            return None;
        }
        let mut result = Vec::with_capacity(len);
        for _ in 0..len {
            result.push(self.recv());
        }
        self.end_transfer();
        Some(result)
    }
}