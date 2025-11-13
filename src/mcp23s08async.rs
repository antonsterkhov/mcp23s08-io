use core::fmt::Debug;

use embedded_hal_async::spi::{Operation, SpiDevice};

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

pub struct Mcp23s08async<SPI> {
    spi: SPI,
    hw_addr: u8,
    olat: u8,
    iodir: u8,
}

impl<SPI, E> Mcp23s08async<SPI>
where
    SPI: SpiDevice<Error = E>,
{

    pub async fn new(mut spi: SPI, hw_addr: u8) -> Result<Self, Error<E>> {
        if hw_addr > 3 {
            return Err(Error::BadAddress);
        }

        let mut this = Self {
            spi,
            hw_addr,
            olat: 0x00,
            iodir: 0xFF,
        };

        // IOCON в дефолт
        this.write_reg(Reg::IOCON, 0x00).await?;
        this.iodir = this.read_reg(Reg::IODIR).await?;
        this.olat = this.read_reg(Reg::OLAT).await?;
        Ok(this)
    }


    pub fn pin<'a>(&'a mut self, pin: Pin) -> GpioPin<'a, SPI> {
        GpioPin { dev: self, pin }
    }

    pub async fn set_pin_direction(&mut self, pin: Pin, input: bool) -> Result<(), Error<E>> {
        if input {
            self.iodir |= pin.bit();
        } else {
            self.iodir &= !pin.bit();
        }
        self.write_reg(Reg::IODIR, self.iodir).await
    }

    pub async fn set_port_direction(&mut self, mask: u8) -> Result<(), Error<E>> {
        self.iodir = mask;
        self.write_reg(Reg::IODIR, mask).await
    }

    pub async fn set_pin_pullup(&mut self, pin: Pin, enable: bool) -> Result<(), Error<E>> {
        let mut gppu = self.read_reg(Reg::GPPU).await?;
        if enable {
            gppu |= pin.bit();
        } else {
            gppu &= !pin.bit();
        }
        self.write_reg(Reg::GPPU, gppu).await
    }

    pub async fn set_port_pullups(&mut self, mask: u8) -> Result<(), Error<E>> {
        self.write_reg(Reg::GPPU, mask).await
    }

    pub async fn set_pin_polarity(
        &mut self,
        pin: Pin,
        pol: Polarity,
    ) -> Result<(), Error<E>> {
        let mut ipol = self.read_reg(Reg::IPOL).await?;
        match pol {
            Polarity::Normal => ipol &= !pin.bit(),
            Polarity::Inverted => ipol |= pin.bit(),
        }
        self.write_reg(Reg::IPOL, ipol).await
    }

    pub async fn read_port(&mut self) -> Result<u8, Error<E>> {
        self.read_reg(Reg::GPIO).await
    }

    pub async fn read_pin(&mut self, pin: Pin) -> Result<bool, Error<E>> {
        Ok(self.read_port().await? & pin.bit() != 0)
    }

    pub async fn write_port(&mut self, value: u8) -> Result<(), Error<E>> {
        self.olat = value;
        self.write_reg(Reg::GPIO, value).await
    }

    pub async fn write_pin(&mut self, pin: Pin, high: bool) -> Result<(), Error<E>> {
        if high {
            self.olat |= pin.bit();
        } else {
            self.olat &= !pin.bit();
        }
        self.write_reg(Reg::GPIO, self.olat).await
    }


    pub async fn write_olat(&mut self, value: u8) -> Result<(), Error<E>> {
        self.olat = value;
        self.write_reg(Reg::OLAT, value).await
    }

    pub async fn set_pin_interrupt_enable(
        &mut self,
        pin: Pin,
        enable: bool,
    ) -> Result<(), Error<E>> {
        let mut gpinten = self.read_reg(Reg::GPINTEN).await?;
        if enable {
            gpinten |= pin.bit();
        } else {
            gpinten &= !pin.bit();
        }
        self.write_reg(Reg::GPINTEN, gpinten).await
    }

    pub async fn set_port_interrupt_enable(&mut self, mask: u8) -> Result<(), Error<E>> {
        self.write_reg(Reg::GPINTEN, mask).await
    }

    pub async fn set_pin_interrupt_mode(
        &mut self,
        pin: Pin,
        mode: InterruptMode,
    ) -> Result<(), Error<E>> {
        let mut intcon = self.read_reg(Reg::INTCON).await?;
        match mode {
            InterruptMode::OnChange => intcon &= !pin.bit(),
            InterruptMode::CompareToDefault => intcon |= pin.bit(),
        }
        self.write_reg(Reg::INTCON, intcon).await
    }

    #[inline]
    fn opcode_write(&self) -> u8 {
        0x40 | ((self.hw_addr & 0x03) << 1)
    }

    #[inline]
    fn opcode_read(&self) -> u8 {
        0x40 | ((self.hw_addr & 0x03) << 1) | 1
    }

    async fn write_reg(&mut self, reg: Reg, val: u8) -> Result<(), Error<E>> {
        let opcode = self.opcode_write();
        let frame = [opcode, reg as u8, val];
        let mut ops = [Operation::Write(&frame)];
        self.spi.transaction(&mut ops).await.map_err(Error::Spi)
    }

    async fn read_reg(&mut self, reg: Reg) -> Result<u8, Error<E>> {
        let opcode = self.opcode_read();
        let cmd = [opcode, reg as u8];
        let mut byte = [0u8; 1];
        let mut ops = [Operation::Write(&cmd), Operation::Read(&mut byte)];
        self.spi.transaction(&mut ops).await.map_err(Error::Spi)?;
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




pub struct GpioPin<'a, SPI> {
    dev: &'a mut Mcp23s08async<SPI>,
    pin: Pin,
}

impl<'a, SPI, E> GpioPin<'a, SPI>
where
    SPI: SpiDevice<Error = E>,
{
    pub async fn is_high(&mut self) -> Result<bool, Error<E>> {
        self.dev.read_pin(self.pin).await
    }

    pub async fn is_low(&mut self) -> Result<bool, Error<E>> {
        Ok(!self.is_high().await?)
    }

    pub async fn set_high(&mut self) -> Result<(), Error<E>> {
        self.dev.write_pin(self.pin, true).await
    }

    pub async fn set_low(&mut self) -> Result<(), Error<E>> {
        self.dev.write_pin(self.pin, false).await
    }

    pub async fn is_set_high(&mut self) -> Result<bool, Error<E>> {
        Ok(self.dev.olat & self.pin.bit() != 0)
    }

    pub async fn is_set_low(&mut self) -> Result<bool, Error<E>> {
        Ok(!self.is_set_high().await?)
    }
}
