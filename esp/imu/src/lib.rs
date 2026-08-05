#![no_std]
#![allow(clippy::future_not_send)]
extern crate alloc;

use alloc::boxed::Box;
use bt_constants::{
    ImuData, ToBytesLe, Tri,
    imu_conf::{FromPrimitive, ImuCommand, ImuStatus, IntoPrimitive},
};
use defmt::info;
use embassy_executor::Spawner;
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel, signal::Signal,
};
use esp_hal::{
    i2c::master::{Config as I2cConfig, I2c},
    peripherals::{GPIO8, GPIO9, I2C0},
};
use mpu6050_dmp::{
    accel::{Accel, AccelF32, AccelFullScale},
    calibration::{CalibrationParameters, CalibrationThreshold},
    gyro::{Gyro, GyroF32, GyroFullScale},
    sensor_async::Mpu6050,
};
use static_cell::StaticCell;
use trouble_host::prelude::*;

#[allow(non_snake_case)]
pub struct I2CConnection {
    pub I2C0: I2C0<'static>,
    pub GPIO8: GPIO8<'static>,
    pub GPIO9: GPIO9<'static>,
}

pub fn setup(
    peripherals: I2CConnection,
    signal: &'static Signal<CriticalSectionRawMutex, ImuData>,
    spawner: &Spawner,
) {
    let i2c = I2c::new(peripherals.I2C0, I2cConfig::default())
        .unwrap()
        .with_sda(peripherals.GPIO8)
        .with_scl(peripherals.GPIO9)
        .into_async();

    spawner.spawn(imu_task(i2c, signal).expect("IMU task"));
}

static MPU: StaticCell<Mpu6050<I2c<'static, esp_hal::Async>>> = StaticCell::new();

const fn accel_to_tri(accel: &AccelF32) -> Tri {
    Tri {
        x: accel.x(),
        y: accel.y(),
        z: accel.z(),
    }
}

const fn gyro_to_tri(gyro: &GyroF32) -> Tri {
    Tri {
        x: gyro.x(),
        y: gyro.y(),
        z: gyro.z(),
    }
}

const ACCEL_SCALE: AccelFullScale = AccelFullScale::G2;
const GYRO_SCALE: GyroFullScale = GyroFullScale::Deg1000;
static COMMAND_CHANNEL: Channel<CriticalSectionRawMutex, ImuCommand, 20> = Channel::new();
pub static STATUS_CHANNEL: Signal<CriticalSectionRawMutex, [u8; 32]> = Signal::new();

struct Status {
    calibration: (Accel, Gyro),
    status: ImuStatus,
}

impl Status {
    fn signal(&self) {
        let mut status = [0; 32];
        status[0] = self.status.into();
        for (i, n) in [
            self.calibration.0.x(),
            self.calibration.0.y(),
            self.calibration.0.z(),
            self.calibration.1.x(),
            self.calibration.1.y(),
            self.calibration.1.z(),
        ]
        .into_iter()
        .enumerate()
        {
            status[(i * 2 + 1)..(i * 2 + 3)].copy_from_slice(&n.to_le_bytes());
        }
        STATUS_CHANNEL.signal(status);
    }

    const fn calibration(&self) -> (Accel, Gyro) {
        self.calibration
    }

    const fn set_calibration(&mut self, calibration: (Accel, Gyro)) {
        self.calibration = calibration;
    }

    const fn status(&self) -> ImuStatus {
        self.status
    }

    const fn set_status(&mut self, status: ImuStatus) {
        self.status = status;
    }

    const fn new(accel: Accel, gyro: Gyro) -> Self {
        Self {
            calibration: (accel, gyro),
            status: ImuStatus::Initializing,
        }
    }
}

#[allow(clippy::large_stack_frames)] // 4137 B
#[embassy_executor::task]
pub async fn imu_task(
    i2c: I2c<'static, esp_hal::Async>,
    signal: &'static Signal<CriticalSectionRawMutex, ImuData>,
) {
    let mut status = Status::new(Accel::new(0, 0, 0), Gyro::new(0, 0, 0));
    // let mut calibration = (Accel::new(0, 0, 0), Gyro::new(0, 0, 0));
    // status[0] = ImuStatus::Initializing.into();
    // STATUS_CHANNEL.signal(status);
    status.signal();

    let sensor = match Box::pin(Mpu6050::new(i2c, mpu6050_dmp::address::Address::default())).await {
        Ok(s) => s,
        Err(e) => {
            defmt::error!("MPU6050 init failed: {:?}", defmt::Debug2Format(&e));
            panic!("mpu init failed");
        }
    };
    let mpu = MPU.init(sensor); // full struct moved out of the async fn's own state
    mpu.initialize_dmp(&mut embassy_time::Delay).await.unwrap(); // or skip DMP, use raw accel/gyro reads
    mpu.set_accel_full_scale(ACCEL_SCALE).await.unwrap();
    mpu.set_gyro_full_scale(GYRO_SCALE).await.unwrap();

    mpu.set_accel_calibration(&status.calibration().0)
        .await
        .unwrap();
    mpu.set_gyro_calibration(&status.calibration().1)
        .await
        .unwrap();

    status.set_status(ImuStatus::Ok);
    status.signal();

    // if option_env!("CALIBRATE").is_some() {
    //     let data = mpu
    //         .calibrate(
    //             &mut embassy_time::Delay,
    //             &CalibrationParameters {
    //                 accel_scale: ACCEL_SCALE,
    //                 accel_threshold: CalibrationThreshold::from_accel_scale(ACCEL_SCALE),
    //                 gyro_scale: GYRO_SCALE,
    //                 gyro_threshold: CalibrationThreshold::from_gyro_scale(GYRO_SCALE),
    //                 warmup_iterations: 10,
    //                 iterations: 20,
    //                 gravity: mpu6050_dmp::calibration::ReferenceGravity::XP,
    //             },
    //         )
    //         .await
    //         .unwrap();
    //     info!("Calibrated: {:?} {:?}", data.0, data.1);

    //     status[0] = ImuStatus::Ok.into();
    //     STATUS_CHANNEL.signal(status);
    // }

    loop {
        while let Ok(cmd) = COMMAND_CHANNEL.try_receive() {
            match cmd {
                ImuCommand::Calibrate => {
                    status.set_status(ImuStatus::Calibrating);
                    status.signal();
                    let data = mpu
                        .calibrate(
                            &mut embassy_time::Delay,
                            &CalibrationParameters {
                                accel_scale: ACCEL_SCALE,
                                accel_threshold: CalibrationThreshold::from_accel_scale(
                                    ACCEL_SCALE,
                                ),
                                gyro_scale: GYRO_SCALE,
                                gyro_threshold: CalibrationThreshold::from_gyro_scale(GYRO_SCALE),
                                warmup_iterations: 10,
                                iterations: 20,
                                gravity: mpu6050_dmp::calibration::ReferenceGravity::XP,
                            },
                        )
                        .await
                        .unwrap();
                    status.set_status(ImuStatus::Ok);
                    status.set_calibration(data);
                    status.signal();
                    info!("Calibrated: {:?} {:?}", data.0, data.1);
                }
                _ => {}
            }
        }
        let accel = Box::pin(mpu.accel()).await.unwrap().scaled(ACCEL_SCALE);
        let gyro = Box::pin(mpu.gyro())
            .await
            .unwrap()
            .scaled(mpu6050_dmp::gyro::GyroFullScale::Deg250);
        // info!("[IMU] Accel: {}", defmt::Debug2Format(&accel));
        // info!("[IMU] Gyro: {}", defmt::Debug2Format(&gyro));
        signal.signal(ImuData {
            accel: accel_to_tri(&accel),
            gyro: gyro_to_tri(&gyro),
        });
        // push sample to your data channel
        embassy_time::Timer::after_millis(10).await; // ~100Hz poll
    }
}

#[gatt_service(uuid = bt_constants::MOTION_SERVICE_UUID)]
pub struct MotionService {
    #[characteristic(uuid = bt_constants::IMU_DATA_CHAR_UUID, read, notify, value = [0; 24])]
    pub imu_data: [u8; 24],
    #[characteristic(uuid = bt_constants::IMU_CONF_CHAR_UUID, write)]
    pub imu_conf: [u8; 8],
    #[characteristic(uuid = bt_constants::IMU_STATUS_CHAR_UUID, read, notify, value = [0; 32])]
    pub imu_status: [u8; 32],
}

pub fn handle_write_events(
    event: &WriteEvent<'_, '_, DefaultPacketPool>,
    motion: &MotionService,
) -> bool {
    let handle = event.handle();

    if handle == motion.imu_conf.handle {
        let command = ImuCommand::from_primitive(event.data()[0]);
        let data = &event.data()[1..];
        match command {
            ImuCommand::Calibrate => {
                info!("Calibrating!");
                COMMAND_CHANNEL.try_send(ImuCommand::Calibrate).unwrap();
            }
            _ => (),
        }
        true
    } else {
        false
    }
}
