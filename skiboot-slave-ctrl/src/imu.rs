use alloc::boxed::Box;
use defmt::info;
use embassy_executor::Spawner;
use esp_hal::{
    i2c::master::{Config as I2cConfig, I2c},
    peripherals::{GPIO8, GPIO9, I2C0},
};
use mpu6050_dmp::sensor_async::Mpu6050;
use static_cell::StaticCell;

#[allow(non_snake_case)]
pub struct I2CConnection {
    pub I2C0: I2C0<'static>,
    pub GPIO8: GPIO8<'static>,
    pub GPIO9: GPIO9<'static>,
}

pub fn setup(peripherals: I2CConnection, spawner: &Spawner) {
    let i2c = I2c::new(peripherals.I2C0, I2cConfig::default())
        .unwrap()
        .with_sda(peripherals.GPIO8)
        .with_scl(peripherals.GPIO9)
        .into_async();

    spawner.spawn(imu_task(i2c).expect("IMU task"));
}

static MPU: StaticCell<Mpu6050<I2c<'static, esp_hal::Async>>> = StaticCell::new();

#[allow(clippy::large_stack_frames)] // 4137 B
#[embassy_executor::task]
pub async fn imu_task(i2c: I2c<'static, esp_hal::Async>) {
    let sensor =
        match Box::pin(Mpu6050::new(i2c, mpu6050_dmp::address::Address::default())).await {
            Ok(s) => s,
            Err(e) => {
                defmt::error!("MPU6050 init failed: {:?}", defmt::Debug2Format(&e));
                panic!("mpu init failed");
            }
        };
    let mpu = MPU.init(sensor); // full struct moved out of the async fn's own state
    mpu.initialize_dmp(&mut embassy_time::Delay).await.unwrap(); // or skip DMP, use raw accel/gyro reads
    loop {
        let accel = Box::pin(mpu.accel()).await.unwrap();
        let gyro = Box::pin(mpu.gyro()).await.unwrap();
        info!("[IMU] Accel: {}", defmt::Debug2Format(&accel));
        info!("[IMU] Gyro: {}", defmt::Debug2Format(&gyro));
        // push sample to your data channel
        //embassy_time::Timer::after_millis(10).await; // ~100Hz poll
        embassy_time::Timer::after_millis(5000).await; // ~100Hz poll
    }
}
