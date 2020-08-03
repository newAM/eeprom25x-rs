//! Inspired by [eeprom24x-rs], this is a driver for the [Microchip 25AA02E48]
//! SPI EEPROM, based on the [`embedded-hal`] traits.
//!
//! This EEPROM is unique because it has an EUI-48 MAC address programmed into
//! the EEPROM, which is convient for creating internet connected devices valid
//! MAC addresses.
//!
//! ## FTDI Example
//!
//! Instructions are provided for a Debian based OS.
//!
//! 1. Run `sudo apt install libclang-dev libftdi1-dev`.
//! 2. Create a file `/etc/udev/rules.d/99-libftdi.rules`
//! 3. Put the following test into the file:
//!
//! ```text
//! SUBSYSTEMS=="usb", ATTRS{idVendor}=="0403", ATTRS{idProduct}=="6001", GROUP="dialout", MODE="0660"
//! SUBSYSTEMS=="usb", ATTRS{idVendor}=="0403", ATTRS{idProduct}=="6014", GROUP="dialout", MODE="0660"
//! ```
//!
//! 4. Reload udev `sudo udevadm control --reload-rules && sudo udevadm trigger`.
//! 5. Build the binary `cargo build --example ftdi`.
//!
//! I used the [adafruit FT232H breakout], create the following connections.
//!
//! * Connect SCK to D0
//! * Connect MOSI to D1
//! * Connect MISO to D2
//! * Connect CS to C0
//! * Connect Vdd to 3.3V or 5V
//! * Connect Vss to GND
//!
//! Run the example with `cargo run --example ftdi`.
//!
//! [adafruit FT232H breakout]: https://www.adafruit.com/product/2264
//! [eeprom24x-rs]: https://github.com/eldruin/eeprom24x-rs
//! [Microchip 25AA02E48]: http://ww1.microchip.com/downloads/en/DeviceDoc/25AA02E48-25AA02E64-2K-SPI-Bus-Serial-EEPROM-Data%20Sheet_DS20002123G.pdf
//! [`embedded-hal`]: https://github.com/rust-embedded/embedded-hal
#![deny(missing_docs, unsafe_code)]
#![no_std]

use embedded_hal as hal;

use hal::blocking;
use hal::digital::v2::OutputPin;

/// Read instruction.
pub const INSTRUCTION_READ: u8 = 0x03;
/// Write instruction.
pub const INSTRUCTION_WRITE: u8 = 0x02;
/*
const INSTRUCTION_WRDI: u8 = 0x04;
const INSTRUCTION_WREN: u8 = 0x06;
const INSTRUCTION_RDSR: u8 = 0x05;
const INSTRUCTION_WRSR: u8 = 0x01;
*/

/// Number of bytes in an EUI48 MAC address.
pub const EUI48_BYTES: usize = 6;
/// EPPROM memory address of the EUI48 address.
pub const EUI48_MEMORY_ADDRESS: u8 = 0xFA;
/// EEPROM page size in bytes.
pub const PAGE_SIZE: usize = 16;
/// Maximum EEPROM address.
pub const MAX_ADDR: usize = 0xFF;

/// Eeprom25aa02e48 driver.
#[derive(Default)]
pub struct Eeprom25aa02e48<SPI, CS> {
    /// SPI device.
    spi: SPI,
    /// GPIO for chip select.
    cs: CS,
}

/// Eeprom25aa02e48 error type.
#[derive(Debug)]
pub enum Error<SpiError, PinError> {
    /// SPI bus error wrapper.
    Spi(SpiError),
    /// GPIO pin error wrapper.
    Pin(PinError),
}

impl<SPI, CS, SpiError, PinError> Eeprom25aa02e48<SPI, CS>
where
    SPI: blocking::spi::Transfer<u8, Error = SpiError> + blocking::spi::Write<u8, Error = SpiError>,
    CS: OutputPin<Error = PinError>,
{
    /// Creates a new `Eeprom25aa02e48` driver from a SPI peripheral
    /// and a chip select digital I/O pin.
    pub fn new(spi: SPI, cs: CS) -> Self {
        Eeprom25aa02e48 { spi: spi, cs: cs }
    }

    fn chip_enable(&mut self) -> Result<(), Error<SpiError, PinError>> {
        self.cs.set_low().map_err(Error::Pin)
    }

    fn chip_disable(&mut self) -> Result<(), Error<SpiError, PinError>> {
        self.cs.set_high().map_err(Error::Pin)
    }

    /// Read from the EEPROM.
    /// The size of the `data` buffer determines the number of bytes read.
    pub fn read_data(
        &mut self,
        address: u8,
        data: &mut [u8],
    ) -> Result<(), Error<SpiError, PinError>> {
        // address is invalid
        assert!(address as usize + data.len() - 1 <= MAX_ADDR);
        let cmd: [u8; 2] = [INSTRUCTION_READ, address];
        self.chip_enable()?;
        let mut spi_functions = || -> Result<(), SpiError> {
            self.spi.write(&cmd)?;
            self.spi.transfer(data)?;
            Ok(())
        };
        let result = spi_functions().map_err(Error::Spi);
        self.chip_disable()?;
        result
    }

    /// Write a byte to the EEPROM.
    pub fn write_byte(&mut self, address: u8, data: u8) -> Result<(), Error<SpiError, PinError>> {
        let cmd: [u8; 3] = [INSTRUCTION_WRITE, address, data];
        self.chip_enable()?;
        let result = self.spi.write(&cmd).map_err(Error::Spi);
        self.chip_disable()?;
        result
    }

    /// Write a page to the EEPROM.
    ///
    /// *Note*: The address must be page aligned.
    pub fn write_page(
        &mut self,
        address: u8,
        data: [u8; PAGE_SIZE],
    ) -> Result<(), Error<SpiError, PinError>> {
        // address not page aligned
        assert!(address % PAGE_SIZE as u8 == 0);
        let cmd: [u8; 2] = [INSTRUCTION_WRITE, address];
        self.chip_enable()?;
        let mut spi_functions = || -> Result<(), SpiError> {
            self.spi.write(&cmd)?;
            self.spi.write(&data)
        };
        let result = spi_functions().map_err(Error::Spi);
        self.chip_disable()?;
        result
    }

    /// Read the EUI48 address from the EEPROM.
    pub fn read_eui48(
        &mut self,
        eui48: &mut [u8; EUI48_BYTES],
    ) -> Result<(), Error<SpiError, PinError>> {
        self.read_data(EUI48_MEMORY_ADDRESS, eui48)
    }
}
