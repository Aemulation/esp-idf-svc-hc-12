use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::{peripheral::Peripheral, prelude::Peripherals};

use anyhow::Result;
use hc_12::{BaudRate, TransmissionMode};

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

    let mut hc_12 = hc_12::Hc12::new(uart1, pin17, pin16, pin4, None)?;

    log::info!("Resetting the hc-12");
    hc_12.command()?.set_default()?;

    let baud = hc_12.command()?.auto_baud()?;
    log::info!("Baud rate set to: {} ", u32::from(baud));

    for i in 0..10 {
        match hc_12.command()?.test() {
            Ok(_) => log::info!("test {i}: success"),
            error @ Err(_) => {
                log::error!("test {i}: ERROR");
                error?;
            }
        }
    }

    for baud_rate in [
        BaudRate::Baud1200,
        BaudRate::Baud2400,
        BaudRate::Baud4800,
        BaudRate::Baud9600,
        BaudRate::Baud19200,
        BaudRate::Baud38400,
        BaudRate::Baud57600,
        BaudRate::Baud115200,
    ] {
        log::info!("Setting baudrate to {}", u32::from(&baud_rate));
        hc_12.command()?.set_baud(&baud_rate)?;

        FreeRtos::delay_ms(200);
        hc_12.command()?.test()?;

        FreeRtos::delay_ms(100);
    }

    hc_12.command()?.test()?;

    FreeRtos::delay_ms(1_000);

    log::info!("Setting transmission mode to FU1");
    hc_12
        .command()?
        .set_transmission_mode(&TransmissionMode::Fu1)?;
    FreeRtos::delay_ms(10_000);

    log::info!("Setting transmission mode to FU2");
    hc_12
        .command()?
        .set_transmission_mode(&TransmissionMode::Fu2)?;
    FreeRtos::delay_ms(10_000);

    hc_12.command()?.test()?;

    log::info!("Setting transmission mode to FU3");
    hc_12
        .command()?
        .set_transmission_mode(&TransmissionMode::Fu3)?;
    FreeRtos::delay_ms(10_000);

    log::info!("Setting transmission mode to FU4");
    hc_12
        .command()?
        .set_transmission_mode(&TransmissionMode::Fu4)?;
    FreeRtos::delay_ms(10_000);

    Ok(())
}
