use std::time::{Duration, Instant};

use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::gpio::{AnyIOPin, AnyOutputPin, InputPin, Output, OutputPin, PinDriver};
use esp_idf_svc::hal::peripheral::Peripheral;
use esp_idf_svc::hal::uart::{self, Uart, UartDriver};

use anyhow::Result;
use esp_idf_svc::hal::units::Hertz;
use esp_idf_svc::sys::EspError;

#[derive(thiserror::Error, Debug)]
enum Hc12Error {
    #[error("Test command did not return OK")]
    Test,
    #[error("Failed to set the requested baud rate")]
    BaudRate,
    #[error("Failed to automatically set the baud rate")]
    AutoBaudRate,
    #[error("Failed to set the requested transmission mode")]
    TransmissionMode,

    #[error("Failed reset")]
    Default,
}

pub enum TransmissionMode {
    Fu1,
    Fu2,
    Fu3,
    Fu4,
}

impl From<&TransmissionMode> for u32 {
    fn from(transmission_mode: &TransmissionMode) -> Self {
        match transmission_mode {
            TransmissionMode::Fu1 => 1,
            TransmissionMode::Fu2 => 2,
            TransmissionMode::Fu3 => 3,
            TransmissionMode::Fu4 => 4,
        }
    }
}

impl From<TransmissionMode> for u32 {
    fn from(transmission_mode: TransmissionMode) -> Self {
        match transmission_mode {
            TransmissionMode::Fu1 => 1,
            TransmissionMode::Fu2 => 2,
            TransmissionMode::Fu3 => 3,
            TransmissionMode::Fu4 => 4,
        }
    }
}

#[derive(Default, Clone, Copy)]
pub enum BaudRate {
    Baud1200,
    Baud2400,
    Baud4800,
    #[default]
    Baud9600,
    Baud19200,
    Baud38400,
    Baud57600,
    Baud115200,
}

impl From<&BaudRate> for &str {
    fn from(value: &BaudRate) -> Self {
        match value {
            BaudRate::Baud1200 => "1200",
            BaudRate::Baud2400 => "2400",
            BaudRate::Baud4800 => "4800",
            BaudRate::Baud9600 => "9600",
            BaudRate::Baud19200 => "19200",
            BaudRate::Baud38400 => "38400",
            BaudRate::Baud57600 => "57600",
            BaudRate::Baud115200 => "115200",
        }
    }
}

impl From<BaudRate> for u32 {
    fn from(baud_rate: BaudRate) -> Self {
        match baud_rate {
            BaudRate::Baud1200 => 1200,
            BaudRate::Baud2400 => 2400,
            BaudRate::Baud4800 => 4800,
            BaudRate::Baud9600 => 9600,
            BaudRate::Baud19200 => 19200,
            BaudRate::Baud38400 => 38400,
            BaudRate::Baud57600 => 57600,
            BaudRate::Baud115200 => 115200,
        }
    }
}

impl From<&BaudRate> for u32 {
    fn from(baud_rate: &BaudRate) -> Self {
        match baud_rate {
            BaudRate::Baud1200 => 1200,
            BaudRate::Baud2400 => 2400,
            BaudRate::Baud4800 => 4800,
            BaudRate::Baud9600 => 9600,
            BaudRate::Baud19200 => 19200,
            BaudRate::Baud38400 => 38400,
            BaudRate::Baud57600 => 57600,
            BaudRate::Baud115200 => 115200,
        }
    }
}

impl From<BaudRate> for Hertz {
    fn from(baud_rate: BaudRate) -> Self {
        u32::from(baud_rate).into()
    }
}

impl From<&BaudRate> for Hertz {
    fn from(baud_rate: &BaudRate) -> Self {
        u32::from(baud_rate).into()
    }
}

pub struct Command<'d, 'h> {
    hc_12: &'h mut Hc12<'d>,
}

impl Drop for Command<'_, '_> {
    fn drop(&mut self) {
        self.hc_12
            .set
            .set_high()
            .expect("Could not exit command mode");
        self.hc_12.last_command_exit = Instant::now();
    }
}

impl<'d, 'h> Command<'d, 'h> {
    fn new(hc_12: &'h mut Hc12<'d>) -> Result<Self> {
        if let Some(time_to_wait) =
            Duration::from_millis(201).checked_sub(hc_12.last_command_exit.elapsed())
        {
            FreeRtos::delay_ms(time_to_wait.as_millis() as u32);
        }
        hc_12.set.set_low()?;
        FreeRtos::delay_ms(200);

        Ok(Self { hc_12 })
    }

    fn send_command(&mut self, command: &str) -> Result<String> {
        let mut buffer = [0u8; 14];
        self.hc_12.driver.clear_rx()?;

        self.hc_12.write(command.as_bytes())?;
        FreeRtos::delay_ms(200);

        let bytes_read = self.hc_12.read(&mut buffer, 200)?;

        Ok(String::from_utf8_lossy(&buffer[..bytes_read]).into_owned())
    }

    pub fn test(&mut self) -> Result<()> {
        let result = self.send_command("AT")?;
        if result != "OK\r\n" {
            return Err(Hc12Error::Test.into());
        }

        Ok(())
    }

    pub fn auto_baud(&mut self) -> Result<BaudRate> {
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
            self.hc_12.driver.change_baudrate(u32::from(baud_rate))?;

            if self.test().is_ok() {
                return Ok(baud_rate);
            }
        }

        Err(Hc12Error::AutoBaudRate.into())
    }

    pub fn set_baud(mut self, baud_rate: &BaudRate) -> Result<()> {
        let command = format!("AT+B{}", u32::from(baud_rate));
        let result = self.send_command(&command)?;

        if result != format!("OK+B{}\r\n", u32::from(baud_rate)) {
            return Err(Hc12Error::BaudRate.into());
        }

        self.hc_12.driver.change_baudrate(u32::from(baud_rate))?;

        Ok(())
    }

    pub fn set_transmission_mode(mut self, transmission_mode: &TransmissionMode) -> Result<()> {
        let command = format!("AT+FU{}", u32::from(transmission_mode));
        let result = self.send_command(&command)?;

        if !result.contains(&format!("OK+FU{}", u32::from(transmission_mode))) {
            return Err(Hc12Error::TransmissionMode.into());
        }

        if let Some(new_baud_rate) = result.split(",").nth(1) {
            let new_baud_rate = new_baud_rate[1..].trim();
            self.hc_12
                .driver
                .change_baudrate(str::parse::<u32>(new_baud_rate)?)?;
        }

        Ok(())
    }

    pub fn set_default(&mut self) -> Result<()> {
        let result = self.send_command("AT+DEFAULT")?;

        if result != ("OK+DEFAULT\r\n") {
            return Err(Hc12Error::Default.into());
        }

        Ok(())
    }
}

pub struct Hc12<'d> {
    driver: UartDriver<'d>,
    set: PinDriver<'d, AnyOutputPin, Output>,

    last_command_exit: Instant,
}

impl<'d> Hc12<'d> {
    pub fn new(
        uart: impl Peripheral<P = impl Uart> + 'd,
        tx: impl Peripheral<P = impl OutputPin> + 'd,
        rx: impl Peripheral<P = impl InputPin> + 'd,
        set: impl Peripheral<P = impl OutputPin> + 'd,
        baud_rate: Option<BaudRate>,
    ) -> Result<Self> {
        let uart_config = uart::config::Config::default().baudrate(BaudRate::Baud9600.into());

        let driver = uart::UartDriver::new(
            uart,
            tx,
            rx,
            Option::<AnyIOPin>::None,
            Option::<AnyIOPin>::None,
            &uart_config,
        )?;

        let mut set = PinDriver::output(set.into_ref().map_into())?;
        set.set_high()?;
        let last_command_exit = Instant::now();

        let mut hc_12 = Self {
            driver,
            set,
            last_command_exit,
        };

        if let Some(baud_rate) = &baud_rate {
            hc_12.driver.change_baudrate(baud_rate)?;
        } else {
            hc_12.command()?.auto_baud()?;
        }

        Ok(hc_12)
    }

    pub fn command<'h>(&'h mut self) -> Result<Command<'d, 'h>> {
        Command::new(self)
    }

    pub fn read(&self, buf: &mut [u8], timeout: u32) -> Result<usize, EspError> {
        self.driver.read(buf, timeout)
    }

    pub fn write(&self, buf: &[u8]) -> Result<usize, EspError> {
        self.driver.write(buf)
    }
}
