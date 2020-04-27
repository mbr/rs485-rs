//! RS485 support for serial devices
//!
//! RS485 is a low-level specification for data transfer. While the spec only
//! defines electrical parameter and very little else, in reality it is most
//! often used for serial data transfers.
//!
//! To realize an RS485 connection, a machine's UARTs are usually used. These
//! support sending and receiving, each through a dedicated pin for RX
//! (receive) and TX (transmit). RS232 can be directly connected this way to
//! allow full duplex (simultaneous send and receive) connections.
//!
//! RS485 differs from RS232 in an important aspect: Instead of dedicating a
//! single line to send and another one to receive, two wires each are used
//! to transport a differential signal. In combination with higher voltage
//! levels and twisted-pair cabling, this allows for much more reliable
//! transmission results.
//!
//! A working full-duplex RS485 connection requires a transceiver chip and
//! four wires, two for RX and TX. To reduce the number of wires back down to
//! two, a suitable protocol can be used to establish a bi-directional
//! half-duplex connection. Commonly, a "master" device will turn on its
//! line driver, send a request, turn it back off and wait for a reply.
//!
//! Most transceivers have a pins dedicated to turning the linw driver on and
//! off. Being able to turn the driver on just before sending and back off
//! after the transmission is complete is a requirement for implementing these
//! protocols.
//!
//! Often a UART's RTS (request-to-send) pin is connected in a way that
//! makes the transceiver enable the line driver when RTS is on but the
//! receiver instead when RTS off.
//! Since this functionality is common, kernel serial drivers usually support
//! turning RTS on/off. This crate allows configuring that functionality,
//! provided it is setup correctly.
//!
//! When running into issues with RS485, verify that the RTS pin of your UART
//! is actually connected to the transceiver, properly pinmuxed (if necessary)
//! and that the UART itself is enabled.

use libc::c_ulong;
use std::{mem, io};
use std::os::unix::io::{AsRawFd, RawFd};

// constants stolen from C libs
const TIOCSRS485: c_ulong = 0x542f;
const TIOCGRS485: c_ulong = 0x542e;

#[derive(Copy, Clone, Debug)]
pub struct Rs485Flags {
    bits: u32,
}

impl Rs485Flags {
    pub const SER_RS485_ENABLED: Self = Rs485Flags { bits: (1 << 0) };
    pub const SER_RS485_RTS_ON_SEND: Self = Rs485Flags { bits: (1 << 1) };
    pub const SER_RS485_RTS_AFTER_SEND: Self = Rs485Flags { bits: (1 << 2) };
    pub const SER_RS485_RX_DURING_TX: Self = Rs485Flags { bits: (1 << 4) };
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
/// RS485 serial configuration
///
/// Internally, this structure is the same as a [`struct serial_rs485`]
///(http://elixir.free-electrons.com/linux/latest/ident/serial_rs485).
pub struct SerialRs485 {
    flags: Rs485Flags,
    delay_rts_before_send: u32,
    delay_rts_after_send: u32,
    _padding: [u32; 5],
}

impl SerialRs485 {
    /// Create a new, empty set of serial settings
    ///
    /// All flags will default to "off", delays will be set to 0 ms.
    #[inline]
    pub fn new() -> SerialRs485 {
        unsafe { mem::zeroed() }
    }

    #[inline]
    pub fn default() -> SerialRs485 {
        SerialRs485{
            flags : Rs485Flags::SER_RS485_ENABLED,
            delay_rts_before_send : 0,
            delay_rts_after_send : 0,
            _padding : [0u32; 5]
        }
    }

    /// Load settings from file descriptor
    ///
    /// Settings will be loaded from the file descriptor, which must be a
    /// valid serial device support RS485 extensions
    #[inline]
    pub fn from_fd(fd: RawFd) -> io::Result<SerialRs485> {
        let mut conf = SerialRs485::new();

        let rval = unsafe { libc::ioctl(fd, TIOCGRS485, &mut conf as *mut SerialRs485) };

        if rval == -1 {
            return Err(io::Error::last_os_error());
        }

        Ok(conf)
    }

    /// Enable RS485 support
    ///
    /// Unless enabled, none of the settings set take effect.
    #[inline]
    pub fn set_enabled<'a>(&'a mut self, enabled: bool) -> &'a mut Self {
        if enabled {
            self.flags.bits |= Rs485Flags::SER_RS485_ENABLED.bits;
        } else {
            self.flags.bits &= !Rs485Flags::SER_RS485_ENABLED.bits;
        }

        self
    }

    /// Set RTS high or low before sending
    ///
    /// RTS will be set before sending, this setting controls whether
    /// it will be set high (`true`) or low (`false`).
    #[inline]
    pub fn set_rts_on_send<'a>(&'a mut self, rts_on_send: bool) -> &'a mut Self {
        if rts_on_send {
            self.flags.bits |= Rs485Flags::SER_RS485_RTS_ON_SEND.bits;
        } else {
            self.flags.bits &= !Rs485Flags::SER_RS485_RTS_ON_SEND.bits;
        }

        self
    }

    /// Set RTS high or low after sending
    ///
    /// RTS will be set after sending, this setting contrls whether
    /// it will be set high (`true`) or low (`false`).
    #[inline]
    pub fn set_rts_after_send<'a>(&'a mut self, rts_after_send: bool) -> &'a mut Self {
        if rts_after_send {
            self.flags.bits |= Rs485Flags::SER_RS485_RTS_AFTER_SEND.bits;
        } else {
            self.flags.bits &= !Rs485Flags::SER_RS485_RTS_AFTER_SEND.bits;
        }

        self
    }

    /// Delay before sending in ms
    ///
    /// If set to non-zero, transmission will not start until
    /// `delays_rts_before_send` milliseconds after RTS has been set
    #[inline]
    pub fn delay_rts_before_send_ms<'a>(&'a mut self, delay_rts_before_send: u32) -> &'a mut Self {
        self.delay_rts_before_send = delay_rts_before_send;
        self
    }

    /// Hold RTS after sending, in ms
    ///
    /// If set to non-zero, RTS will be kept high/low for
    /// `delays_rts_after_send` ms after the transmission is complete
    #[inline]
    pub fn delay_rts_after_send_ms<'a>(&'a mut self, delay_rts_after_send: u32) -> &'a mut Self {
        self.delay_rts_after_send = delay_rts_after_send;
        self
    }

    /// Allow receiving whilst transmitting
    ///
    /// Note that turning off this option sometimes seems to make the UART
    /// misbehave and cut off transmission. For this reason, it is best left on
    /// even when using half-duplex.
    pub fn set_rx_during_tx<'a>(&'a mut self, set_rx_during_tx: bool) -> &'a mut Self {
        if set_rx_during_tx {
            self.flags.bits |= Rs485Flags::SER_RS485_RX_DURING_TX.bits;
        } else {
            self.flags.bits &= !Rs485Flags::SER_RS485_RX_DURING_TX.bits;
        }
        self
    }

    /// Apply settings to file descriptor
    ///
    /// Applies the constructed configuration a raw filedescriptor using
    /// `ioctl`.
    #[inline]
    pub fn set_on_fd(&self, fd: RawFd) -> io::Result<()> {
        let rval = unsafe { libc::ioctl(fd, TIOCSRS485, self as *const SerialRs485) };

        if rval == -1 {
            return Err(io::Error::last_os_error());
        }

        Ok(())
    }
}


/// Rs485 controls
///
/// A convenient trait for controlling Rs485 parameters.
pub trait Rs485 {
    /// Retrieves RS485 parameters from target
    fn get_rs485_conf(&self) -> io::Result<SerialRs485>;

    /// Sets RS485 parameters on target
    fn set_rs485_conf(&self, conf: &SerialRs485) -> io::Result<()>;

    /// Update RS485 configuration
    ///
    /// Combines `get_rs485_conf` and `set_rs485_conf` through a closure
    fn update_rs485_conf<F: FnOnce(&mut SerialRs485) -> ()>(&self, f: F) -> io::Result<()>;
}

impl<T: AsRawFd> Rs485 for T {
    #[inline]
    fn get_rs485_conf(&self) -> io::Result<SerialRs485> {
        SerialRs485::from_fd(self.as_raw_fd())
    }

    #[inline]
    fn set_rs485_conf(&self, conf: &SerialRs485) -> io::Result<()> {
        conf.set_on_fd(self.as_raw_fd())
    }

    #[inline]
    fn update_rs485_conf<F: FnOnce(&mut SerialRs485) -> ()>(&self, f: F) -> io::Result<()> {
        let mut conf = self.get_rs485_conf()?;
        f(&mut conf);
        self.set_rs485_conf(&conf)
    }
}
