use defmt::{info, warn};
use embassy_executor::Spawner;
use esp_hal::{
    peripherals::{GPIO20, GPIO21, UART1},
    uart::{Config as UartConfig, Uart},
};
use nmea0183::{ParseResult, Parser};

#[allow(non_snake_case)]
pub struct GPSConnection {
    pub UART1: UART1<'static>,
    pub GPIO20: GPIO20<'static>,
    pub GPIO21: GPIO21<'static>,
}

pub fn setup(peripherals: GPSConnection, spawner: &Spawner) {
    let uart_gps = Uart::new(peripherals.UART1, UartConfig::default().with_baudrate(9600))
        .unwrap()
        .with_rx(peripherals.GPIO20)
        .with_tx(peripherals.GPIO21) // PA1010D doesn't need TX, but esp-hal often wants the pin bound
        .into_async();

    spawner.spawn(gps_task(uart_gps).expect("Correct GPS task"));
}

#[allow(clippy::large_stack_frames)] // 1037 bytes
#[embassy_executor::task]
pub async fn gps_task(mut uart: Uart<'static, esp_hal::Async>) {
    let mut parser = Parser::new();
    let mut buf = [0u8; 128];
    loop {
        let n = match embedded_io_async::Read::read(&mut uart, &mut buf).await {
            Ok(n) => n,
            Err(e) => {
                warn!("[GPS] UART read error: {}", defmt::Debug2Format(&e));
                continue; // drop this chunk, keep the task alive
            }
        };
		
        info!("[GPS] read: {=[u8]:a}", &buf[..n]);
        for result in parser.parse_from_bytes(&buf[..n]) {
            match result {
                Ok(ParseResult::RMC(Some(rmc))) => {
                    // rmc.latitude, rmc.longitude, rmc.speed, rmc.datetime — push to your data channel
                    info!("[GPS] RMC: {}", defmt::Debug2Format(&rmc));
                }
                Ok(ParseResult::GGA(Some(gga))) => {
                    // altitude, fix quality, satellite count
                    info!("[GPS] GGA: {}", defmt::Debug2Format(&gga));
                }
                Ok(other) => {
                    info!("[GPS] {}", defmt::Debug2Format(&other))
                }
                Err(e) => {
                    warn!("[GPS] Error: {}", defmt::Debug2Format(&e))
                }
            }
        }
    }
}
