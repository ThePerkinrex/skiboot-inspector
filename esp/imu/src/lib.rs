#![no_std]
#![allow(clippy::future_not_send)]
extern crate alloc;

use alloc::boxed::Box;
use bt_constants::{
    ByteSize, ImuData, ToBytesLe, Tri, imu_conf::{FromPrimitive, ImuCommand, ImuStatus, IntoPrimitive},
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
    accel::{Accel, AccelF32, AccelFullScale}, calibration::{CalibrationParameters, CalibrationThreshold}, config::DigitalLowPassFilter, gravity::Gravity, gyro::{Gyro, GyroF32, GyroFullScale}, quaternion::Quaternion, sensor_async::Mpu6050, yaw_pitch_roll::YawPitchRoll,
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

const fn ypr_to_tri(ypr: &YawPitchRoll) -> Tri {
    Tri {
        x: ypr.yaw,
        y: ypr.pitch,
        z: ypr.roll,
    }
}

const fn gravity_to_tri(gravity: &Gravity) -> Tri {
    Tri {
        x: gravity.x,
        y: gravity.y,
        z: gravity.z,
    }
}

const ACCEL_SCALE: AccelFullScale = AccelFullScale::G2;
const GYRO_SCALE: GyroFullScale = GyroFullScale::Deg250;
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
    let mut status = Status::new(Accel::new(-3399, -3054, 947), Gyro::new(123, -46, -24));
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
    mpu.set_sample_rate_divider(14).await.unwrap();
    mpu.set_digital_lowpass_filter(DigitalLowPassFilter::Filter1)
        .await
        .unwrap();

    mpu.set_accel_full_scale(ACCEL_SCALE).await.unwrap();
    mpu.set_gyro_full_scale(GYRO_SCALE).await.unwrap();

    mpu.set_accel_calibration(&status.calibration().0)
        .await
        .unwrap();
    mpu.set_gyro_calibration(&status.calibration().1)
        .await
        .unwrap();
    mpu.initialize_dmp(&mut embassy_time::Delay).await.unwrap();
    // mpu.enable_fifo().await.unwrap();

    status.set_status(ImuStatus::Ok);
    status.signal();

    let mut fifo_buf = [0u8; 28];
    let mut i = 0;

    loop {
        while let Ok(cmd) = COMMAND_CHANNEL.try_receive() {
            match cmd {
                ImuCommand::Calibrate => {
                    status.set_status(ImuStatus::Calibrating);
                    info!("Calibrating!");
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
                                iterations: 50,
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

        // TODO replace with full DMP FIFO reading. ImuData will be expanded at a later date
        // DMP 28-byte structure
        // [QUAT W][      ][QUAT X][      ][QUAT Y][      ][QUAT Z][      ]
        //   0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15

        // [GYRO X][GxYRO Y][GYRO Z][ACC X ][ACC Y ][ACC Z ]
        //  16  17  18  19  20  21  22  23  24  25  26  27
        // let accel = Box::pin(mpu.accel()).await.unwrap().scaled(ACCEL_SCALE);
        // let gyro = Box::pin(mpu.gyro())
        //     .await
        //     .unwrap()
        //     .scaled(mpu6050_dmp::gyro::GyroFullScale::Deg250);
        // // info!("[IMU] Accel: {}", defmt::Debug2Format(&accel));
        // // info!("[IMU] Gyro: {}", defmt::Debug2Format(&gyro));
        // signal.signal(ImuData {
        //     accel: accel_to_tri(&accel),
        //     gyro: gyro_to_tri(&gyro),
        // });

        // Check if enough data is available in the FIFO buffer
        let fifo_count = mpu.get_fifo_count().await.unwrap_or(0);

        if fifo_count >= 1024 {
            // Reset FIFO if buffer overflows
            let _ = mpu.reset_fifo().await;
        } else {
            while fifo_count >= fifo_buf.len() {
                if mpu.read_fifo(&mut fifo_buf).await.is_ok() {
                    // info!("Read FIFO");
                    // Extract Quaternion (scaled floating-point [-1.0, 1.0])
                    // info!("quat: {}", Quaternion::from_bytes(&fifo_buf[..16]));
                    if let Some(quat) = Quaternion::from_bytes(&fifo_buf[..16]) {
                        // Calculate gravity vector from quaternion
                        // let gravity = mpu6050_dmp::gravity::Gravity::from(quat);

                        // // Calculate Yaw, Pitch, Roll in radians
                        // let ypr = YawPitchRoll::from(quat);

                        // If you still need calibrated accel/gyro along with orientation:
                        let gyro = Gyro::from_bytes(fifo_buf[16..22].try_into().unwrap())
                            .scaled(GYRO_SCALE);
                        let accel = Accel::from_bytes(fifo_buf[22..28].try_into().unwrap())
                            .scaled(ACCEL_SCALE);

                        signal.signal(ImuData {
                            accel: accel_to_tri(&accel),
                            gyro: gyro_to_tri(&gyro),
                            quat: [quat.w, quat.y, quat.y, quat.z]
                            // Add your calculated orientation fields to ImuData here, e.g.:
                            // yaw: ypr.yaw,
                            // pitch: ypr.pitch,
                            // roll: ypr.roll,
                        });

                        i += 1;
                        if i > 1000 {
                            info!(
                                "{:04} {}, {}, {}",
                                &fifo_count, quat, accel, gyro
                            );
                            i = 0;
                        }
                    }
                }
            }
        }
        // info!("{:04}",fifo_count);

        // push sample to your data channel
        embassy_time::Timer::after_millis(10).await; // ~100Hz poll
    }
}

#[gatt_service(uuid = bt_constants::MOTION_SERVICE_UUID)]
pub struct MotionService {
    #[characteristic(uuid = bt_constants::IMU_DATA_CHAR_UUID, read, notify, value = [0; ImuData::SIZE])]
    pub imu_data: [u8; ImuData::SIZE],
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
