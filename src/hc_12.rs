use std::fmt::Write;
use std::time::{Duration, Instant};

use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::gpio::{AnyIOPin, AnyOutputPin, InputPin, Output, OutputPin, PinDriver};
use esp_idf_svc::hal::peripheral::Peripheral;
use esp_idf_svc::hal::peripheral::PeripheralRef;
use esp_idf_svc::hal::uart::{self, Uart, UartDriver};

use anyhow::Result;
use esp_idf_svc::hal::units::Hertz;

#[derive(thiserror::Error, Debug)]
enum Hc12Error {
    #[error("Test command did not return OK")]
    Test,
    #[error("Failed to set the requested baud rate")]
    BaudRate,
    #[error("Failed to set the requested transmission mode")]
    TransmissionMode,
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

#[derive(Default)]
pub enum Baudrate {
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

impl From<&Baudrate> for &str {
    fn from(value: &Baudrate) -> Self {
        match value {
            Baudrate::Baud1200 => "1200",
            Baudrate::Baud2400 => "2400",
            Baudrate::Baud4800 => "4800",
            Baudrate::Baud9600 => "9600",
            Baudrate::Baud19200 => "19200",
            Baudrate::Baud38400 => "38400",
            Baudrate::Baud57600 => "57600",
            Baudrate::Baud115200 => "115200",
        }
    }
}

impl From<Baudrate> for u32 {
    fn from(baud_rate: Baudrate) -> Self {
        match baud_rate {
            Baudrate::Baud1200 => 1200,
            Baudrate::Baud2400 => 2400,
            Baudrate::Baud4800 => 4800,
            Baudrate::Baud9600 => 9600,
            Baudrate::Baud19200 => 19200,
            Baudrate::Baud38400 => 38400,
            Baudrate::Baud57600 => 57600,
            Baudrate::Baud115200 => 115200,
        }
    }
}

impl From<&Baudrate> for u32 {
    fn from(baud_rate: &Baudrate) -> Self {
        match baud_rate {
            Baudrate::Baud1200 => 1200,
            Baudrate::Baud2400 => 2400,
            Baudrate::Baud4800 => 4800,
            Baudrate::Baud9600 => 9600,
            Baudrate::Baud19200 => 19200,
            Baudrate::Baud38400 => 38400,
            Baudrate::Baud57600 => 57600,
            Baudrate::Baud115200 => 115200,
        }
    }
}

impl From<Baudrate> for Hertz {
    fn from(baud_rate: Baudrate) -> Self {
        u32::from(baud_rate).into()
    }
}

impl From<&Baudrate> for Hertz {
    fn from(baud_rate: &Baudrate) -> Self {
        u32::from(baud_rate).into()
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
        set: PeripheralRef<'d, AnyOutputPin>,
        baud_rate: Option<Baudrate>,
    ) -> Result<Self> {
        let uart_config = uart::config::Config::default().baudrate(Baudrate::Baud9600.into());

        let driver = uart::UartDriver::new(
            uart,
            tx,
            rx,
            Option::<AnyIOPin>::None,
            Option::<AnyIOPin>::None,
            &uart_config,
        )?;

        let mut set = PinDriver::output(set)?;
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
            hc_12.determine_baud()?;
        }

        Ok(hc_12)
    }

    fn send_command(&mut self, command: &str) -> Result<String> {
        if let Some(time_to_wait) =
            Duration::from_millis(200).checked_sub(self.last_command_exit.elapsed())
        {
            FreeRtos::delay_ms(time_to_wait.as_millis() as u32);
        }

        let mut buffer = [0u8; 12];
        let bytes_read = (|| -> Result<usize> {
            self.set.set_low()?;
            self.driver.clear_rx()?;
            FreeRtos::delay_ms(80);

            self.driver.write_str(command)?;
            FreeRtos::delay_ms(80);
            let bytes_read = self.driver.read(&mut buffer, 200)?;
            self.set.set_high()?;
            Ok(bytes_read)
        })();

        self.last_command_exit = Instant::now();
        let bytes_read = bytes_read?;

        eprintln!("raw buffer: {:?}", &buffer[0..bytes_read]);

        Ok(String::from_utf8_lossy(&buffer[..bytes_read]).into_owned())
    }

    pub fn determine_baud(&mut self) -> Result<Baudrate> {
        for baud_rate in [
            Baudrate::Baud1200,
            Baudrate::Baud2400,
            Baudrate::Baud4800,
            Baudrate::Baud9600,
            Baudrate::Baud19200,
            Baudrate::Baud38400,
            Baudrate::Baud57600,
            Baudrate::Baud115200,
        ] {
            eprintln!("Testing baud: {}", u32::from(&baud_rate));
            self.driver.change_baudrate(u32::from(&baud_rate))?;
            if self.test().is_ok() {
                return Ok(baud_rate);
            }
        }

        Err(Hc12Error::BaudRate.into())
    }

    pub fn test(&mut self) -> Result<()> {
        let result = self.send_command("AT")?;
        if result != "OK\r\n" {
            return Err(Hc12Error::Test.into());
        }

        Ok(())
    }

    pub fn set_baud(&mut self, baud_rate: &Baudrate) -> Result<()> {
        let command = format!("AT+B{}", u32::from(baud_rate));
        let result = self.send_command(&command)?;

        eprintln!("Result: {result}");
        if result != format!("OK+B{}\r\n", u32::from(baud_rate)) {
            return Err(Hc12Error::BaudRate.into());
        }

        self.driver.change_baudrate(u32::from(baud_rate))?;

        Ok(())
    }

    pub fn set_transmission_mode(&mut self, transmission_mode: &TransmissionMode) -> Result<()> {
        let command = format!("AT+FU{}", u32::from(transmission_mode));
        let result = self.send_command(&command)?;

        if result != format!("OK+FU{}\r\n", u32::from(transmission_mode)) {
            return Err(Hc12Error::TransmissionMode.into());
        }

        Ok(())
    }
}
