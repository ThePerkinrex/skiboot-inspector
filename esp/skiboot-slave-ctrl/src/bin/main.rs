#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use bt_constants::{ImuData, ToBytesLe};
use bt_hci::controller::ExternalController;
use defmt::info;
use embassy_executor::Spawner;
use embassy_futures::select::select;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::timer::timg::TimerGroup;
use esp_println as _;
use esp_radio::ble::controller::BleConnector;
use imu::{self, I2CConnection};
use static_cell::StaticCell;
use trouble_host::prelude::*;

extern crate alloc;

const SLOTS: usize = 20;
const CONNECTIONS_MAX: usize = 1;
const L2CAP_CHANNELS_MAX: usize = 1;
static RESOURCES: StaticCell<
    HostResources<DefaultPacketPool, CONNECTIONS_MAX, L2CAP_CHANNELS_MAX>,
> = StaticCell::new();
static STACK: StaticCell<
    Stack<'static, ExternalController<BleConnector<'static>, SLOTS>, DefaultPacketPool>,
> = StaticCell::new();

static IMU_SIGNAL: Signal<CriticalSectionRawMutex, ImuData> = Signal::new();

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

const EVENT_HANDLERS: &[fn(&WriteEvent<'_, '_, DefaultPacketPool>, &Server) -> bool] =
    &[|e, s| imu::handle_write_events(e, &s.motion_service)];

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    // generator version: 1.3.0
    // generator parameters: --chip esp32c3 -o esp32c3-mini-1 -o unstable-hal -o embassy -o alloc -o ble-trouble -o defmt -o esp-backtrace -o wokwi -o vscode -o stable-x86_64-unknown-linux-gnu

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    // The following pins are used to bootstrap the chip. They are available
    // for use, but check the datasheet of the module for more information on them.
    // - GPIO2
    // - GPIO8
    // - GPIO9
    // These GPIO pins are in use by some feature of the module and should not be used.
    let _ = peripherals.GPIO11;
    let _ = peripherals.GPIO12;
    let _ = peripherals.GPIO13;
    let _ = peripherals.GPIO14;
    let _ = peripherals.GPIO15;
    let _ = peripherals.GPIO16;
    let _ = peripherals.GPIO17;

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 66320);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_interrupt =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);

    info!("Embassy initialized!");

    // find more examples https://github.com/embassy-rs/trouble/tree/main/examples/esp32
    let transport = BleConnector::new(peripherals.BT, Default::default()).unwrap();
    let ble_controller = ExternalController::<_, SLOTS>::new(transport);
    let resources = RESOURCES.init(HostResources::new());
    let stack = STACK.init(trouble_host::new(ble_controller, resources));
    let host = stack.build();

    let server = Server::new_with_config(GapConfig::Peripheral(PeripheralConfig {
        name: "SkiBoot Slave",
        appearance: &appearance::outdoor_sports_activity::GENERIC_OUTDOOR_SPORTS_ACTIVITY,
    }))
    .unwrap();

    let mut peripheral = host.peripheral;

    spawner.spawn(ble_task(host.runner).unwrap());

    // gps::setup(GPSConnection{ UART1: peripherals.UART1, GPIO20: peripherals.GPIO20, GPIO21: peripherals.GPIO21 }, &spawner);
    imu::setup(
        I2CConnection {
            I2C0: peripherals.I2C0,
            GPIO8: peripherals.GPIO8,
            GPIO9: peripherals.GPIO9,
        },
        &IMU_SIGNAL,
        &spawner,
    );

    // TODO: Spawn some tasks
    let _ = spawner;

    let mut adv_data = [0u8; 31];

    let len = AdStructure::encode_slice(
        &[
            AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
            AdStructure::CompleteLocalName(b"SkiBoot Slave"),
        ],
        &mut adv_data,
    )
    .expect("encode ad");
    let mut error_count = 0;

    loop {
        info!("Advertising!");
        match peripheral
            .advertise(
                &Default::default(),
                Advertisement::ConnectableScannableUndirected {
                    adv_data: &adv_data[..len],
                    scan_data: &[],
                },
            )
            .await
        {
            Ok(advertiser) => {
                info!("Advertised!");
                let conn = advertiser
                    .accept()
                    .await
                    .unwrap()
                    .with_attribute_server(&server)
                    .unwrap();
                info!("Connected");
                let imu = server.motion_service.imu_data;
                select(
                    async {
                        loop {
                            match conn.next().await {
                                GattConnectionEvent::Disconnected { .. } => break,
                                GattConnectionEvent::Gatt { event } => {
                                    if let GattEvent::Write(w) = &event {
                                        for h in EVENT_HANDLERS {
                                            if h(w, &server) {
                                                break;
                                            }
                                        }
                                    }
                                    event.accept().ok();
                                }
                                _ => {}
                            }
                        }
                    },
                    select(
                        async {
                            loop {
                                let data = IMU_SIGNAL.wait().await;
                                imu.notify(&conn, &data.to_bytes_le()).await.unwrap();
                            }
                        },
                        async {
                            loop {
                                let data = imu::STATUS_CHANNEL.wait().await;
                                server
                                    .motion_service
                                    .imu_status
                                    .notify(&conn, &data)
                                    .await
                                    .unwrap();
                            }
                        },
                    ),
                )
                .await;
            }
            Err(e) => {
                defmt::error!("advertise failed: {:?}", e);
                error_count += 1;
                if error_count > 5 {
                    panic!("Error count exceded");
                }
            }
        }
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.1.0/examples
}

#[embassy_executor::task]
async fn ble_task(
    mut runner: trouble_host::prelude::Runner<
        'static,
        ExternalController<BleConnector<'static>, SLOTS>,
        DefaultPacketPool,
    >,
) {
    loop {
        runner.run().await.ok();
    }
}

#[gatt_server]
struct Server {
    motion_service: imu::MotionService,
}
