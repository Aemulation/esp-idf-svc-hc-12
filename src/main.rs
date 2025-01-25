use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::gpio::AnyIOPin;
use esp_idf_svc::hal::uart;
use esp_idf_svc::hal::{peripheral::Peripheral, prelude::Peripherals};

use anyhow::Result;
use hc_12::{Baudrate, TransmissionMode};

mod hc_12;

fn main() -> Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take()?;
    let pin4 = peripherals.pins.gpio4.into_ref();
    let pin16 = peripherals.pins.gpio16.into_ref();
    let pin17 = peripherals.pins.gpio17.into_ref();
    let uart1 = peripherals.uart1.into_ref();

    log::info!("Hello, world!");

    let mut hc_12 = hc_12::Hc12::new(uart1, pin17, pin16, pin4.map_into(), None)?;

    for i in 0..10 {
        match hc_12.test() {
            Ok(_) => log::info!("test {i}: success"),
            Err(_) => log::error!("test {i}: ERROR"),
        }
    }

    // let baud_rate1 = Baudrate::Baud115200;
    // log::info!("Setting baudrate to {}", u32::from(&baud_rate1));
    // hc_12.set_baud(&baud_rate1)?;
    //
    // let baud_rate2 = Baudrate::Baud9600;
    // log::info!("Setting baudrate to {}", u32::from(&baud_rate2));
    // hc_12.set_baud(&baud_rate2)?;

    log::info!("Setting transmission mode to FU1");
    hc_12.set_transmission_mode(&TransmissionMode::Fu1)?;
    FreeRtos::delay_ms(10_000);

    log::info!("Setting transmission mode to FU2");
    hc_12.set_transmission_mode(&TransmissionMode::Fu2)?;
    FreeRtos::delay_ms(10_000);

    log::info!("Setting transmission mode to FU3");
    hc_12.set_transmission_mode(&TransmissionMode::Fu3)?;
    FreeRtos::delay_ms(10_000);

    log::info!("Setting transmission mode to FU4");
    hc_12.set_transmission_mode(&TransmissionMode::Fu4)?;
    FreeRtos::delay_ms(10_000);

    Ok(())
}
