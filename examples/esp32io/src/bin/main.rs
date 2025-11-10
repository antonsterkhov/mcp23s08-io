#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

use embedded_hal_bus::spi::ExclusiveDevice;
use esp_hal::clock::CpuClock;
use esp_hal::gpio::{Level, Output, OutputConfig};
use esp_hal::main;
use esp_hal::spi::Mode;
use esp_hal::spi::master::{Config, Spi};
use esp_hal::time::{Duration, Instant, Rate};
use esp_hal::delay::Delay;
use mcp23s08_io::mcp23s08::{Mcp23s08, Pin};
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[main]
fn main() -> ! {
    // generator version: 1.0.1

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let _peripherals = esp_hal::init(config);

    let cs = Output::new(_peripherals.GPIO5, Level::High, OutputConfig::default());

    let spi_conf = Config::default()
        .with_mode(Mode::_0)
        .with_frequency(Rate::from_mhz(40));

    let spi = Spi::new(_peripherals.SPI2, spi_conf)
        .unwrap()
        .with_sck(_peripherals.GPIO17)
        .with_mosi(_peripherals.GPIO4)
        .with_miso(_peripherals.GPIO3);


    let d = Delay::new();

    let spi_divice = ExclusiveDevice::new(spi,cs,d).unwrap();


    let mcp = Mcp23s08::new(spi_divice,0).unwrap();


    loop {
        let delay_start = Instant::now();
        while delay_start.elapsed() < Duration::from_millis(500) {}
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.0.0/examples/src/bin
}
