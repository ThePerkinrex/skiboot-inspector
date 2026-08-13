use bt_constants::imu_conf::{ImuCommand, ImuStatus};
use bt_constants::{ByteSize, FromBytesLe, ImuData};
use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter, WriteType};
use btleplug::platform::{Adapter, Manager, Peripheral};
use std::time::Duration;
use tokio::time;
use tokio_stream::StreamExt;
use tracing::{error, info};

// #[derive(Debug)]
// struct TriI16 {
//     x: i16,
//     y: i16,
//     z: i16,
// }

// impl FromBytesLe<6> for TriI16 {
//     fn from_le_bytes(x: &[u8; 6]) -> Self {
//         Self {
//             x: i16::from_le_bytes(x[0..2].try_into().unwrap()),
//             y: i16::from_le_bytes(x[2..4].try_into().unwrap()),
//             z: i16::from_le_bytes(x[4..6].try_into().unwrap()),
//         }
//     }
// }

type TriI16 = [i16; 3];

#[derive(Debug)]
struct Status {
    status: ImuStatus,
    accel: TriI16,
    gyro: TriI16,
}

impl ByteSize for Status {
    const SIZE: usize = 32;
}

impl FromBytesLe for Status {
    fn from_le_bytes(x: &[u8]) -> Self {
        Self {
            status: ImuStatus::from(x[0]),
            accel: TriI16::from_le_bytes(&x[1..7]),
            gyro: TriI16::from_le_bytes(&x[7..13]),
        }
    }
}

pub async fn run() -> anyhow::Result<()> {
    let manager = Manager::new().await.unwrap();

    // get the first bluetooth adapter
    let adapters = manager.adapters().await?;
    let central = adapters.into_iter().nth(0).unwrap();

    // start scanning for devices
    central.start_scan(ScanFilter::default()).await?;
    // instead of waiting, you can use central.events() to get a stream which will
    // notify you of new devices, for an example of that see examples/event_driven_discovery.rs
    time::sleep(Duration::from_secs(2)).await;

    // find the device we're interested in
    let light = find_boot(&central).await.unwrap();

    // connect to the device
    light.connect().await?;
    info!("Connected!");

    info!("Searching for: {}", bt_constants::IMU_DATA_CHAR_UUID);

    // discover services and characteristics
    light.discover_services().await?;
    for c in light.characteristics() {
        info!("C: {}", c.uuid);
    }

    // find the characteristic we want
    let chars = light.characteristics();
    let data_char = chars
        .iter()
        .find(|c| c.uuid == bt_constants::IMU_DATA_CHAR_UUID)
        .unwrap()
        .clone();
    let conf_char = chars
        .iter()
        .find(|c| c.uuid == bt_constants::IMU_CONF_CHAR_UUID)
        .unwrap()
        .clone();
    let status_char = chars
        .iter()
        .find(|c| c.uuid == bt_constants::IMU_STATUS_CHAR_UUID)
        .unwrap()
        .clone();

    light.subscribe(&status_char).await.expect("subscribe");
    light.subscribe(&data_char).await.expect("subscribe");

    let a = light.read(&status_char).await.unwrap();
    let status = Status::from_le_bytes(&a);
    info!("STATUS: {:?}", status);

    let light_clone = light.clone();
    let conf_char_clone = conf_char.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(5)).await;
        info!("Calibrating");
        let mut data = [0u8; 8];
        data[0] = ImuCommand::Calibrate.into();
        if let Err(e) = light_clone
            .write(&conf_char_clone, &data, WriteType::WithResponse)
            .await
        {
            error!("Failed to write calibrate command: {e}");
        }
        info!("Command sent!");
    });

    let mut nots = light.notifications().await.expect("not stream");
    let mut status_count = -2;
    while let Some(a) = nots.next().await {
        if a.uuid == bt_constants::IMU_DATA_CHAR_UUID {
            let imu = ImuData::from_le_bytes(&a.value);
            info!("IMU: {:?}", imu);
            if status_count >= 0 {
                status_count += 1;
            }
        } else if a.uuid == bt_constants::IMU_STATUS_CHAR_UUID {
            let status = Status::from_le_bytes(&a.value);
            info!("STATUS: {:?}", status);
            if status.status == ImuStatus::Calibrating {
                status_count = -1;
            } else if status.status == ImuStatus::Ok && status_count == -1 {
                status_count = 0;
            }
        } else {
            info!("NOT: {} {} {:?}", a.service_uuid, a.uuid, a.value)
        }
        // if status_count > 10 {
        //     exit(0);
        // }
    }

    // dance party
    // let mut rng = rng();
    // for _ in 0..20 {
    //     // let color_cmd = vec![0x56, rng.random(), rng.random(), rng.random(), 0x00, 0xF0, 0xAA];
    //     light.write(&cmd_char, &color_cmd, WriteType::WithoutResponse).await?;
    //     time::sleep(Duration::from_millis(200)).await;
    // }
    Ok(())
}

async fn find_boot(central: &Adapter) -> Option<Peripheral> {
    for p in central.peripherals().await.unwrap() {
        if p.properties()
            .await
            .unwrap()
            .unwrap()
            .local_name
            .iter()
            .any(|name| name.contains("SkiBoot"))
        {
            return Some(p);
        }
    }
    None
}
