//! NOTE: This test file was created with assistance from ChatGPT (OpenAI).

#![allow(clippy::bool_assert_comparison)]

use embedded_hal_mock::eh1::spi::{Mock as SpiMock, Transaction as SpiTransaction};

// Bring the driver under test into scope.
// If your file is at `src/mcp23s08.rs` (typical), keep this as-is.
// Otherwise, adjust the relative path below.
#[path = "../src/mcp23s08.rs"]
mod mcp23s08;
use mcp23s08::{Error, Mcp23s08, Pin, Polarity};

// Helpers
fn init_expectations_for_new(hw_addr: u8, iodir: u8, olat: u8) -> Vec<SpiTransaction<u8>> {
    let op_wr = 0x40 | ((hw_addr & 0x03) << 1) | 0; // write opcode
    let op_rd = 0x40 | ((hw_addr & 0x03) << 1) | 1; // read opcode

    vec![
        // write IOCON = 0x00
        SpiTransaction::transaction_start(),
        SpiTransaction::write_vec(vec![op_wr, 0x05, 0x00]), // Reg::IOCON = 0x05
        SpiTransaction::transaction_end(),
        // read IODIR
        SpiTransaction::transaction_start(),
        SpiTransaction::write_vec(vec![op_rd, 0x00]), // Reg::IODIR = 0x00
        SpiTransaction::read_vec(vec![iodir]),
        SpiTransaction::transaction_end(),
        // read OLAT
        SpiTransaction::transaction_start(),
        SpiTransaction::write_vec(vec![op_rd, 0x0A]), // Reg::OLAT = 0x0A
        SpiTransaction::read_vec(vec![olat]),
        SpiTransaction::transaction_end(),
    ]
}

#[test]
fn new_ok_initializes_cached_state() {
    let expectations = init_expectations_for_new(0, 0xFF, 0x00);
    let mut spi = SpiMock::new(&expectations);

    let dev = Mcp23s08::new(spi.clone(), 0).expect("new should succeed");

    // Drop driver and finalize expectations on the clone
    drop(dev);
    spi.done();
}

#[test]
fn new_rejects_bad_hw_address() {
    // No SPI transactions expected
    let mut spi = SpiMock::new(&[]);
    let err = Mcp23s08::new(spi.clone(), 4)
        .err()
        .expect("new() should reject invalid hardware address (>=4)");
    match err {
        Error::BadAddress => {}
        other => panic!("unexpected error: {other:?}"),
    }
    spi.done();
}

#[test]
fn write_pin_sets_gpio_and_updates_olat() {
    // Start with OLAT=0x00, set P3 high -> write GPIO=0x08
    let mut expectations = init_expectations_for_new(0, 0xFF, 0x00);
    let op_wr = 0x40; // hw_addr=0

    expectations.extend([
        SpiTransaction::transaction_start(),
        // Reg::GPIO = 0x09, value = 0x08
        SpiTransaction::write_vec(vec![op_wr, 0x09, 0x08]),
        SpiTransaction::transaction_end(),
        // Then write GPIO back to 0x00
        SpiTransaction::transaction_start(),
        SpiTransaction::write_vec(vec![op_wr, 0x09, 0x00]),
        SpiTransaction::transaction_end(),
    ]);

    let mut spi = SpiMock::new(&expectations);
    let mut dev = Mcp23s08::new(spi.clone(), 0).unwrap();

    dev.write_pin(Pin::P3, true).unwrap();
    dev.write_pin(Pin::P3, false).unwrap();

    drop(dev);
    spi.done();
}

#[test]
fn read_pin_reads_gpio_and_returns_bit() {
    // We will read GPIO and return 0b0000_0010 so P1 is high
    let mut expectations = init_expectations_for_new(0, 0xFF, 0x00);
    let op_rd = 0x41; // hw_addr=0

    expectations.extend([
        SpiTransaction::transaction_start(),
        SpiTransaction::write_vec(vec![op_rd, 0x09]), // Reg::GPIO
        SpiTransaction::read_vec(vec![0b0000_0010]),
        SpiTransaction::transaction_end(),
    ]);

    let mut spi = SpiMock::new(&expectations);
    let mut dev = Mcp23s08::new(spi.clone(), 0).unwrap();

    let p1 = dev.read_pin(Pin::P1).unwrap();
    assert_eq!(p1, true);

    drop(dev);
    spi.done();
}

#[test]
fn set_pin_direction_updates_cached_iodir_and_writes_register() {
    // Start IODIR=0xFF, set P0 as output -> new IODIR = 0xFE
    let mut expectations = init_expectations_for_new(0, 0xFF, 0x00);
    let op_wr = 0x40; // hw_addr=0, write

    expectations.extend([
        SpiTransaction::transaction_start(),
        SpiTransaction::write_vec(vec![op_wr, 0x00, 0xFE]), // Reg::IODIR
        SpiTransaction::transaction_end(),
    ]);

    let mut spi = SpiMock::new(&expectations);
    let mut dev = Mcp23s08::new(spi.clone(), 0).unwrap();

    dev.set_pin_direction(Pin::P0, /*input=*/ false).unwrap();

    drop(dev);
    spi.done();
}

#[test]
fn set_pin_polarity_reads_modifies_and_writes_ipol() {
    // Read IPOL -> 0x00, set P2 inverted -> write 0x04
    let mut expectations = init_expectations_for_new(0, 0xFF, 0x00);
    let op_wr = 0x40;
    let op_rd = 0x41;
    expectations.extend([
        // read IPOL
        SpiTransaction::transaction_start(),
        SpiTransaction::write_vec(vec![op_rd, 0x01]), // Reg::IPOL
        SpiTransaction::read_vec(vec![0x00]),
        SpiTransaction::transaction_end(),
        // write IPOL
        SpiTransaction::transaction_start(),
        SpiTransaction::write_vec(vec![op_wr, 0x01, 0x04]),
        SpiTransaction::transaction_end(),
    ]);

    let mut spi = SpiMock::new(&expectations);
    let mut dev = Mcp23s08::new(spi.clone(), 0).unwrap();

    dev.set_pin_polarity(Pin::P2, Polarity::Inverted).unwrap();

    drop(dev);
    spi.done();
}

#[test]
fn read_interrupt_flags_and_capture() {
    let mut expectations = init_expectations_for_new(0, 0xFF, 0x00);
    let op_rd = 0x41;
    expectations.extend([
        // INTF
        SpiTransaction::transaction_start(),
        SpiTransaction::write_vec(vec![op_rd, 0x07]),
        SpiTransaction::read_vec(vec![0xAA]),
        SpiTransaction::transaction_end(),
        // INTCAP
        SpiTransaction::transaction_start(),
        SpiTransaction::write_vec(vec![op_rd, 0x08]),
        SpiTransaction::read_vec(vec![0x55]),
        SpiTransaction::transaction_end(),
    ]);

    let mut spi = SpiMock::new(&expectations);
    let mut dev = Mcp23s08::new(spi.clone(), 0).unwrap();

    assert_eq!(dev.read_interrupt_flags().unwrap(), 0xAA);
    assert_eq!(dev.read_interrupt_capture().unwrap(), 0x55);

    drop(dev);
    spi.done();
}

#[test]
fn set_int_open_drain_and_polarity() {
    // iocon read-modify-write for ODR and INTPOL bits
    let mut expectations = init_expectations_for_new(0, 0xFF, 0x00);
    let op_wr = 0x40; let op_rd = 0x41;

    // set_int_open_drain(true): read IOCON -> 0, write 0b0000_0100
    expectations.extend([
        SpiTransaction::transaction_start(),
        SpiTransaction::write_vec(vec![op_rd, 0x05]),
        SpiTransaction::read_vec(vec![0x00]),
        SpiTransaction::transaction_end(),
        SpiTransaction::transaction_start(),
        SpiTransaction::write_vec(vec![op_wr, 0x05, 0x04]),
        SpiTransaction::transaction_end(),
    ]);

    // set_int_polarity(active_high=true): read IOCON -> 0x04, write 0x06
    expectations.extend([
        SpiTransaction::transaction_start(),
        SpiTransaction::write_vec(vec![op_rd, 0x05]),
        SpiTransaction::read_vec(vec![0x04]),
        SpiTransaction::transaction_end(),
        SpiTransaction::transaction_start(),
        SpiTransaction::write_vec(vec![op_wr, 0x05, 0x06]),
        SpiTransaction::transaction_end(),
    ]);

    let mut spi = SpiMock::new(&expectations);
    let mut dev = Mcp23s08::new(spi.clone(), 0).unwrap();

    dev.set_int_open_drain(true).unwrap();
    dev.set_int_polarity(true).unwrap();

    drop(dev);
    spi.done();
}
