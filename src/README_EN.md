# MCP23S08 Driver (SPI GPIO Expander) — Documentation (EN)

This file describes the public API of the `Mcp23s08` driver (Microchip **MCP23S08**, an 8‑bit GPIO expander over **SPI**) and helper types from `mcp23s08.rs`. The driver is built on **embedded‑hal 1.0** traits and is compatible with `SpiDevice`.

## Table of Contents
- [Supported Peripherals](#supported-peripherals)
- [Errors](#errors)
- [Enums](#enums)
- [Core Types](#core-types)
- [Methods of `Mcp23s08`](#methods-of-mcp23s08)
- [Pin Interface `GpioPin`](#pin-interface-gpiopin)
- [Usage Examples](#usage-examples)
- [Notes on Interrupts](#notes-on-interrupts)
- [MCP23S08 Registers](#mcp23s08-registers)
- [Extended Usage Examples](#extended-usage-examples)

---

## Supported Peripherals

- Chip: **MCP23S08** (SPI variant of the MCP23x08 family).
- Bus: **SPI** via `embedded_hal::spi::SpiDevice`.
- Interrupt logic level and `INT` output mode are configurable.

## Errors

```rust
pub enum Error<SpiE> {
    Spi(SpiE),
    BadAddress,
}
```
- `Spi(SpiE)` — an error propagated from the underlying SPI device.
- `BadAddress` — the `hw_addr` (hardware address) is out of the allowed range (0..=3).

## Enums

```rust
pub enum Pin { P0, P1, P2, P3, P4, P5, P6, P7 }
```
Logical GPIO lines GPIO0..GPIO7.

```rust
pub enum Polarity { Normal, Inverted }
```
Input polarity: normal or inverted (mask of the `IPOL` register).

```rust
pub enum InterruptMode { OnChange, CompareToDefault }
```
Interrupt generation mode: on input change (`INTCON=0`) or by comparison with `DEFVAL` (`INTCON=1`).

## Core Types

```rust
pub struct Mcp23s08<SPI> {
    // hidden fields: spi, hw_addr, olat, iodir
}
```
High‑level driver object. Holds a reference to the SPI device, hardware address, and software shadows of the `OLAT` and `IODIR` registers to minimize reads.

```rust
pub struct GpioPin<'a, SPI> {
    // dev: &'a mut Mcp23s08<SPI>,
    // pin: Pin,
}
```
Helper "handle" for a single pin. Implements `embedded_hal::digital::{InputPin, OutputPin, StatefulOutputPin, ErrorType}` on top of `Mcp23s08` operations.

## Methods of `Mcp23s08`

- `new(spi, hw_addr) -> Result<Self, Error<E>>`  
  Creates the driver. Initializes `IOCON=0x00`, reads current `IODIR` and `OLAT`. Returns `BadAddress` if `hw_addr > 3`.

- `set_pin_direction(pin, input)` / `set_port_direction(mask)`  
  Configure direction: `true` → input, `false` → output (`IODIR`).

- `set_pin_pullup(pin, enable)` / `set_port_pullups(mask)`  
  Enable internal pull‑ups on inputs (`GPPU`).

- `set_pin_polarity(pin, pol)`  
  Input polarity (`IPOL`).

- `read_port() -> u8` / `read_pin(pin) -> bool`  
  Read the state of the port/pin (`GPIO`).

- `write_port(value)` / `write_pin(pin, high)`  
  Write logical levels to the port/pin (`GPIO`). For consistency the software shadow `olat` is also updated.

- `write_olat(value)`  
  Direct write to `OLAT`.

- Interrupt configuration:  
  - `set_pin_interrupt_enable(pin, enable)` / `set_port_interrupt_enable(mask)` → `GPINTEN`  
  - `set_pin_interrupt_mode(pin, mode)` / `set_port_interrupt_mode(mask)` → `INTCON`/`DEFVAL`  
  - `set_port_default_compare(defval)` → `DEFVAL`  
  - `read_interrupt_flags() -> u8` → `INTF`  
  - `read_interrupt_capture() -> u8` → `INTCAP`  
  - `clear_interrupts() -> u8` — read `INTCAP` (clears flags).

- `INT` output configuration:  
  - `set_int_open_drain(enable)` — `IOCON.ODR`  
  - `set_int_polarity(active_high)` — `IOCON.INTPOL`

- Convenience:  
  - `pin(pin) -> GpioPin` — get a handle to a single pin.  
  - `into_inner(self) -> SPI` — extract the underlying SPI device.

## Pin Interface `GpioPin`

Implemented traits (embedded‑hal 1.0):

- `ErrorType<Error = Error<E>>`
- `InputPin`  
  - `is_high()/is_low()` read the `GPIO` register and return the level of the selected pin.
- `OutputPin`  
  - `set_high()/set_low()` modify `OLAT` and write it.  
- `StatefulOutputPin`  
  - `is_set_high()/is_set_low()` use the stored `olat` shadow (no bus read).

> Note: `ToggleableOutputPin` is not implemented, but can be added on top of `OLAT` read/write.

## Usage Examples

```rust
use embedded_hal::spi::SpiDevice;
use your_crate::mcp23s08::{Mcp23s08, Pin, Polarity};

fn init<SPI, E>(spi: SPI) -> Mcp23s08<SPI>
where
    SPI: SpiDevice<Error = E>,
{
    // Hardware address: A2..A0 = 0b001 → hw_addr = 1
    let mut dev = Mcp23s08::new(spi, 1).expect("addr");
    // Direction: P0..P3 — outputs, P4..P7 — inputs
    dev.set_port_direction(0b1111_0000).unwrap();
    // Pull‑ups on inputs
    dev.set_port_pullups(0b1111_0000).unwrap();
    // Normal input polarity
    dev.set_pin_polarity(Pin::P4, Polarity::Normal).unwrap();
    dev
}
```

Working with a single pin via `GpioPin`:

```rust
let mut led = dev.pin(Pin::P0);
led.set_high().unwrap();
assert!(led.is_set_high().unwrap()); // check via OLAT shadow
let level = led.is_high().unwrap();  // actual GPIO read
```

Configuring "on change" interrupts:

```rust
// Enable interrupts on P7..P4
dev.set_port_interrupt_enable(0b1111_0000).unwrap();
// Mode: on level change (INTCON=0 for corresponding bits)
dev.set_port_interrupt_mode(0b0000_0000).unwrap();
// INT — active high and push‑pull
dev.set_int_polarity(true).unwrap();
dev.set_int_open_drain(false).unwrap();
```

Clearing interrupt flags and reading the latch:

```rust
if dev.read_interrupt_flags().unwrap() != 0 {
    let latched = dev.clear_interrupts().unwrap(); // reads INTCAP
    // process latched...
}
```

## Notes on Interrupts

- In `InterruptMode::OnChange` (`INTCON=0`) a flag is set on any input change.
- In `CompareToDefault` mode, comparison is against `DEFVAL`; use `set_port_default_compare`.
- To clear flags you must read `INTCAP`/`GPIO` after the interrupt source.

## MCP23S08 Registers

The driver uses the following registers (addresses in hexadecimal):

```
IODIR(0x00)  IPOL(0x01)  GPINTEN(0x02)  DEFVAL(0x03)
INTCON(0x04) IOCON(0x05) GPPU(0x06)     INTF(0x07)
INTCAP(0x08) GPIO(0x09)  OLAT(0x0A)
```

> Some methods support “port masks” (`u8`) to configure multiple lines in a single call.

---

## Extended Usage Examples

> All examples below are skeletons; adapt them to your HAL/board. The code relies on **embedded‑hal 1.0** traits.

### 1) SPI device initialization (single chip‑select)

```rust
use embedded_hal::spi::SpiDevice;
use your_crate::mcp23s08::Mcp23s08;

fn setup<SPI, E>(spi_dev: SPI) -> Mcp23s08<SPI>
where
    SPI: SpiDevice<Error = E>,
{
    // A2..A0 = 0 → hardware address 0
    let mut mcp = Mcp23s08::new(spi_dev, 0).expect("addr");
    // P0..P3 outputs, P4..P7 inputs with pull‑ups
    mcp.set_port_direction(0b1111_0000).unwrap();
    mcp.set_port_pullups(0b1111_0000).unwrap();
    mcp
}
```

### 2) Creating an `SpiDevice` from a bus and CS (example with embedded‑hal‑bus)

```rust
use embedded_hal::digital::OutputPin;
use embedded_hal::spi::SpiBus;
use embedded_hal_bus::spi::ExclusiveDevice;
use your_crate::mcp23s08::Mcp23s08;

fn new_with_cs<BUS, CS, E>(bus: BUS, cs: CS) -> Mcp23s08<ExclusiveDevice<BUS, CS>>
where
    BUS: SpiBus<Error = E>,
    CS: OutputPin,
{
    // Choose the critical section appropriate for your platform
    let dev = ExclusiveDevice::new(bus, cs, critical_section::with);
    Mcp23s08::new(dev, 0).unwrap()
}
```

### 3) Software blink and read back the level

```rust
use embedded_hal::delay::DelayNs;
use your_crate::mcp23s08::{Mcp23s08, Pin};

fn blink<SPI, E, D: DelayNs>(mcp: &mut Mcp23s08<SPI>, mut delay: D)
where
    SPI: embedded_hal::spi::SpiDevice<Error = E>,
{
    mcp.set_pin_direction(Pin::P0, false).unwrap(); // output
    for _ in 0..5 {
        mcp.write_pin(Pin::P0, true).unwrap();
        delay.delay_ms(200);
        // Read what the port "sees"
        let state = mcp.read_pin(Pin::P0).unwrap();
        assert!(state);
        mcp.write_pin(Pin::P0, false).unwrap();
        delay.delay_ms(200);
    }
}
```

### 4) Debouncing buttons on inputs P4..P7 with pull‑ups

```rust
use your_crate::mcp23s08::Mcp23s08;

fn setup_buttons<SPI, E>(mcp: &mut Mcp23s08<SPI>)
where
    SPI: embedded_hal::spi::SpiDevice<Error = E>,
{
    // P4..P7 inputs with pull‑ups; leave others as is
    let mask_inputs = 0b1111_0000;
    // Direction: 1 = input
    let mut dir = mcp.read_port().unwrap(); // use current as a starting point (optional)
    dir |= mask_inputs;
    mcp.set_port_direction(dir).unwrap();
    mcp.set_port_pullups(mask_inputs).unwrap();
}

fn poll_buttons<SPI, E>(mcp: &mut Mcp23s08<SPI>) -> u8
where
    SPI: embedded_hal::spi::SpiDevice<Error = E>,
{
    // Simple debounce: two reads with a delay; bits that match are considered "stable"
    let a = mcp.read_port().unwrap();
    cortex_m::asm::delay(48_000); // ~1 ms @48 MHz (tune for your platform)
    let b = mcp.read_port().unwrap();
    !(a ^ b) & 0b1111_0000 // "stable" bits on inputs P4..P7 (1 = high level)
}
```

### 5) Hardware interrupts: "on change" on P6..P7

```rust
use your_crate::mcp23s08::Mcp23s08;

fn enable_change_interrupts<SPI, E>(mcp: &mut Mcp23s08<SPI>)
where
    SPI: embedded_hal::spi::SpiDevice<Error = E>,
{
    // Enable interrupts only on P6..P7
    mcp.set_port_interrupt_enable(0b1100_0000).unwrap();
    // Mode: on level change (INTCON=0 for these bits)
    mcp.set_port_interrupt_mode(0b0000_0000).unwrap();
    // INT line — active high, push‑pull
    mcp.set_int_polarity(true).unwrap();
    mcp.set_int_open_drain(false).unwrap();
}

// External interrupt handler (pseudocode):
#[interrupt]
fn EXTI9_5() {
    // … clear the controller's interrupt flag …
    critical_section::with(|cs| {
        let mcp = unsafe { MCP.borrow(cs).as_mut().unwrap() };
        let flags = mcp.read_interrupt_flags().unwrap();
        if flags != 0 {
            let latched = mcp.clear_interrupts().unwrap(); // reads INTCAP
            // pass event to a queue/rtic/embassy/etc.
            process_gpio_event(flags, latched);
        }
    });
}
```

### 6) Compare with DEFVAL: interrupt on deviation from the "normal" state

```rust
use your_crate::mcp23s08::Mcp23s08;

fn enable_compare_interrupts<SPI, E>(mcp: &mut Mcp23s08<SPI>)
where
    SPI: embedded_hal::spi::SpiDevice<Error = E>,
{
    // Consider "high" on all inputs as normal (pulled up)
    mcp.set_port_default_compare(0b1111_0000).unwrap();
    // Compare with DEFVAL on inputs P4..P7 (INTCON=1 for these bits)
    mcp.set_port_interrupt_mode(0b1111_0000).unwrap();
    mcp.set_port_interrupt_enable(0b1111_0000).unwrap();
}
```

### 7) Working with multiple expanders on the same bus

```rust
use embedded_hal::spi::SpiDevice;
use your_crate::mcp23s08::Mcp23s08;

fn two_devices<SPI, E>(spi_a: SPI, spi_b: SPI)
where
    SPI: SpiDevice<Error = E>,
{
    // Option A: one SpiDevice, different hardware addresses (A2..A0) — if you have a single CS wired
    let mut dev0 = Mcp23s08::new(spi_a, 0).unwrap();
    let mut dev1 = Mcp23s08::new(spi_b, 1).unwrap();
    dev0.write_port(0x55).unwrap();
    dev1.write_port(0xAA).unwrap();
}
```

### 8) Port mask operations: set/reset multiple outputs

```rust
use your_crate::mcp23s08::Mcp23s08;

fn set_reset_mask<SPI, E>(mcp: &mut Mcp23s08<SPI>, set_mask: u8, reset_mask: u8)
where
    SPI: embedded_hal::spi::SpiDevice<Error = E>,
{
    // The current OLAT shadow is stored inside the driver, so read‑modify‑write is safe
    let mut olat = mcp.read_port().unwrap(); // you may keep your own shadow as well
    olat |= set_mask;
    olat &= !reset_mask;
    mcp.write_port(olat).unwrap();
}
```

### 9) Using `GpioPin` as a pin handle (output/input)

```rust
use your_crate::mcp23s08::{Mcp23s08, Pin};

fn pin_handle<SPI, E>(mcp: &mut Mcp23s08<SPI>)
where
    SPI: embedded_hal::spi::SpiDevice<Error = E>,
{
    let mut led = mcp.pin(Pin::P0);
    led.set_high().unwrap();
    assert!(led.is_set_high().unwrap()); // check via shadow (no bus)
    // Turn P1 into input and read it
    mcp.set_pin_direction(Pin::P1, true).unwrap();
    let btn = mcp.pin(Pin::P1);
    let level = btn.is_high().unwrap();  // actual GPIO read
    let _ = level;
}
```

### 10) Error handling

```rust
use your_crate::mcp23s08::{Mcp23s08, Error};

fn try_write<SPI, E>(mcp: &mut Mcp23s08<SPI>) -> Result<(), Error<E>>
where
    SPI: embedded_hal::spi::SpiDevice<Error = E>,
{
    mcp.write_port(0x00)?;
    Ok(())
}

// During initialization:
match Mcp23s08::new(spi_dev, 5) {
    Err(Error::BadAddress) => { /* report invalid A2..A0 */ }
    Err(Error::Spi(_e)) => { /* handle bus error */ }
    Ok(mut dev) => { /* … */ }
}
```

### 11) High‑level example: LED "on while button is pressed"

```rust
use embedded_hal::spi::SpiDevice;
use your_crate::mcp23s08::{Mcp23s08, Pin};

fn led_follows_button<SPI, E>(mut dev: Mcp23s08<SPI>)
where
    SPI: SpiDevice<Error = E>,
{
    dev.set_pin_direction(Pin::P0, false).unwrap(); // LED — output
    dev.set_pin_direction(Pin::P4, true).unwrap();  // BTN — input
    dev.set_pin_pullup(Pin::P4, true).unwrap();     // pull‑up on the button
    loop {
        let pressed = dev.read_pin(Pin::P4).unwrap() == false; // active low
        dev.write_pin(Pin::P0, pressed).unwrap();
        // add a small delay if needed
    }
}
```

### 12) Extract the underlying SPI device

```rust
let spi_dev = mcp23s08.into_inner(); // useful for graceful shutdown
```
