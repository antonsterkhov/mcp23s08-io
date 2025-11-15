use core::fmt::Debug;
use embedded_hal::digital::Error as DigitalError;
use embedded_hal::digital::ErrorKind;
use embedded_hal::spi::{Operation, SpiDevice};

use crate::mcp23s08async::GpioPin;

#[derive(Debug)]
pub enum Error<SpiE> {
    Spi(SpiE),
    BadAddress,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pin {
    P0,
    P1,
    P2,
    P3,
    P4,
    P5,
    P6,
    P7,
}
impl Pin {
    #[inline]
    fn bit(self) -> u8 {
        1u8 << (self as u8)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Polarity {
    Normal,
    Inverted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InterruptMode {
    OnChange,

    CompareToDefault,
}

pub struct Mcp23s08<SPI> {
    spi: SPI,
    hw_addr: u8,
    olat: u8,
    iodir: u8,
}

impl<SPI, E> Mcp23s08<SPI>
where
    SPI: SpiDevice<Error = E>,
{
    pub fn new(spi: SPI, hw_addr: u8) -> Result<Self, Error<E>> {
        if hw_addr > 3 {
            return Err(Error::BadAddress);
        }
        let mut this = Self {
            spi,
            hw_addr,
            olat: 0x00,
            iodir: 0xFF,
        };

        this.write_reg(Reg::IOCON, 0x00)?;
        this.iodir = this.read_reg(Reg::IODIR)?;
        this.olat = this.read_reg(Reg::OLAT)?;
        Ok(this)
    }

    pub fn set_pin_direction(&mut self, pin: Pin, input: bool) -> Result<(), Error<E>> {
        if input {
            self.iodir |= pin.bit();
        } else {
            self.iodir &= !pin.bit();
        }
        self.write_reg(Reg::IODIR, self.iodir)
    }

    pub fn set_port_direction(&mut self, mask: u8) -> Result<(), Error<E>> {
        self.iodir = mask;
        self.write_reg(Reg::IODIR, mask)
    }

    pub fn set_pin_pullup(&mut self, pin: Pin, enable: bool) -> Result<(), Error<E>> {
        let mut gppu = self.read_reg(Reg::GPPU)?;
        if enable {
            gppu |= pin.bit();
        } else {
            gppu &= !pin.bit();
        }
        self.write_reg(Reg::GPPU, gppu)
    }

    pub fn set_port_pullups(&mut self, mask: u8) -> Result<(), Error<E>> {
        self.write_reg(Reg::GPPU, mask)
    }

    pub fn set_pin_polarity(&mut self, pin: Pin, pol: Polarity) -> Result<(), Error<E>> {
        let mut ipol = self.read_reg(Reg::IPOL)?;
        match pol {
            Polarity::Normal => ipol &= !pin.bit(),
            Polarity::Inverted => ipol |= pin.bit(),
        }
        self.write_reg(Reg::IPOL, ipol)
    }

    pub fn read_port(&mut self) -> Result<u8, Error<E>> {
        self.read_reg(Reg::GPIO)
    }

    pub fn read_pin(&mut self, pin: Pin) -> Result<bool, Error<E>> {
        Ok(self.read_port()? & pin.bit() != 0)
    }

    pub fn write_port(&mut self, value: u8) -> Result<(), Error<E>> {
        self.olat = value;
        self.write_reg(Reg::GPIO, value)
    }

    pub fn write_pin(&mut self, pin: Pin, high: bool) -> Result<(), Error<E>> {
        if high {
            self.olat |= pin.bit();
        } else {
            self.olat &= !pin.bit();
        }
        self.write_reg(Reg::GPIO, self.olat)
    }

    pub fn write_olat(&mut self, value: u8) -> Result<(), Error<E>> {
        self.olat = value;
        self.write_reg(Reg::OLAT, value)
    }

    pub fn set_pin_interrupt_enable(&mut self, pin: Pin, enable: bool) -> Result<(), Error<E>> {
        let mut gpinten = self.read_reg(Reg::GPINTEN)?;
        if enable {
            gpinten |= pin.bit();
        } else {
            gpinten &= !pin.bit();
        }
        self.write_reg(Reg::GPINTEN, gpinten)
    }

    pub fn set_port_interrupt_enable(&mut self, mask: u8) -> Result<(), Error<E>> {
        self.write_reg(Reg::GPINTEN, mask)
    }

    pub fn set_pin_interrupt_mode(
        &mut self,
        pin: Pin,
        mode: InterruptMode,
    ) -> Result<(), Error<E>> {
        let mut intcon = self.read_reg(Reg::INTCON)?;
        match mode {
            InterruptMode::OnChange => intcon &= !pin.bit(),
            InterruptMode::CompareToDefault => intcon |= pin.bit(),
        }
        self.write_reg(Reg::INTCON, intcon)
    }

    pub fn set_port_interrupt_mode(&mut self, mask: u8) -> Result<(), Error<E>> {
        self.write_reg(Reg::INTCON, mask)
    }

    pub fn set_port_default_compare(&mut self, defval: u8) -> Result<(), Error<E>> {
        self.write_reg(Reg::DEFVAL, defval)
    }

    pub fn read_interrupt_flags(&mut self) -> Result<u8, Error<E>> {
        self.read_reg(Reg::INTF)
    }

    pub fn read_interrupt_capture(&mut self) -> Result<u8, Error<E>> {
        self.read_reg(Reg::INTCAP)
    }

    pub fn clear_interrupts(&mut self) -> Result<u8, Error<E>> {
        self.read_reg(Reg::GPIO)
    }

    pub fn set_int_open_drain(&mut self, enable: bool) -> Result<(), Error<E>> {
        let mut iocon = self.read_reg(Reg::IOCON)?;
        const ODR: u8 = 1 << 2;
        if enable {
            iocon |= ODR;
        } else {
            iocon &= !ODR;
        }
        self.write_reg(Reg::IOCON, iocon)
    }

    pub fn set_int_polarity(&mut self, active_high: bool) -> Result<(), Error<E>> {
        let mut iocon = self.read_reg(Reg::IOCON)?;
        const INTPOL: u8 = 1 << 1;
        if active_high {
            iocon |= INTPOL;
        } else {
            iocon &= !INTPOL;
        }
        self.write_reg(Reg::IOCON, iocon)
    }

    pub fn pin<'a>(&'a mut self, pin: Pin) -> GpioPin<'a, SPI> {
        GpioPin { dev: self, pin }
    }

    pub fn into_inner(self) -> SPI {
        self.spi
    }

    #[inline]
    fn opcode_write(&self) -> u8 {
        0x40 | ((self.hw_addr & 0x03) << 1) | 0
    }
    #[inline]
    fn opcode_read(&self) -> u8 {
        0x40 | ((self.hw_addr & 0x03) << 1) | 1
    }

    fn write_reg(&mut self, reg: Reg, val: u8) -> Result<(), Error<E>> {
        let opcode = self.opcode_write();
        let frame = [opcode, reg as u8, val];

        let mut ops = [Operation::Write(&frame)];
        self.spi.transaction(&mut ops).map_err(Error::Spi)
    }

    fn read_reg(&mut self, reg: Reg) -> Result<u8, Error<E>> {
        let opcode = self.opcode_read();
        let cmd = [opcode, reg as u8];
        let mut byte = [0u8; 1];
        let mut ops = [Operation::Write(&cmd), Operation::Read(&mut byte)];
        self.spi.transaction(&mut ops).map_err(Error::Spi)?;
        Ok(byte[0])
    }
}

#[repr(u8)]
#[derive(Clone, Copy)]
enum Reg {
    IODIR = 0x00,
    IPOL = 0x01,
    GPINTEN = 0x02,
    DEFVAL = 0x03,
    INTCON = 0x04,
    IOCON = 0x05,
    GPPU = 0x06,
    INTF = 0x07,
    INTCAP = 0x08,
    GPIO = 0x09,
    OLAT = 0x0A,
}

impl<E: Debug> DigitalError for Error<E> {
    #[inline]
    fn kind(&self) -> ErrorKind {
        ErrorKind::Other
    }
}

pub struct GpioPin<'a, SPI> {
    dev: &'a mut Mcp23s08<SPI>,
    pin: Pin,
}

impl<'a, SPI, E> embedded_hal::digital::ErrorType for GpioPin<'a, SPI>
where
    SPI: SpiDevice<Error = E>,
    E: Debug,
{
    type Error = Error<E>;
}

impl<'a, SPI, E> embedded_hal::digital::InputPin for GpioPin<'a, SPI>
where
    SPI: SpiDevice<Error = E>,
    E: Debug,
{
    fn is_high(&mut self) -> Result<bool, Self::Error> {
        self.dev.read_pin(self.pin)
    }
    fn is_low(&mut self) -> Result<bool, Self::Error> {
        Ok(!self.is_high()?)
    }
}

impl<'a, SPI, E> embedded_hal::digital::OutputPin for GpioPin<'a, SPI>
where
    SPI: SpiDevice<Error = E>,
    E: Debug,
{
    fn set_high(&mut self) -> Result<(), Self::Error> {
        self.dev.write_pin(self.pin, true)
    }
    fn set_low(&mut self) -> Result<(), Self::Error> {
        self.dev.write_pin(self.pin, false)
    }
}

impl<'a, SPI, E> embedded_hal::digital::StatefulOutputPin for GpioPin<'a, SPI>
where
    SPI: SpiDevice<Error = E>,
    E: Debug,
{
    fn is_set_high(&mut self) -> Result<bool, Self::Error> {
        Ok(self.dev.olat & self.pin.bit() != 0)
    }
    fn is_set_low(&mut self) -> Result<bool, Self::Error> {
        Ok(!self.is_set_high()?)
    }
}

impl<'a, SPI, E> GpioPin<'a, SPI>
where
    SPI: SpiDevice<Error = E>,
    E: Debug,
{
    pub fn toggle(&mut self) -> Result<(), Error<E>> {
        
        let current = self.dev.read_pin(self.pin)?;
        
        self.dev.write_pin(self.pin, !current)
    }
}
