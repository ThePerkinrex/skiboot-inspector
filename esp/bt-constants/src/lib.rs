#![no_std]

pub mod imu_conf;

use core::array::TryFromSliceError;

macro_rules! UUID_DEF {
    ($name:ident = $uuid:literal) => {
        #[cfg(target_arch = "riscv32")]
        pub const $name: [u8; 16] = {let mut a = uuid::uuid!($uuid).into_bytes(); a.reverse(); a};
        #[cfg(not(target_arch = "riscv32"))]
        pub const $name: uuid::Uuid = uuid::uuid!($uuid);
    };
}

UUID_DEF!(BASE_NAMESPACE_UUID = "45e43ca9-ba7d-4769-bc47-34578c000000");
UUID_DEF!(MOTION_SERVICE_UUID = "45e43ca9-ba7d-4769-bc47-34578c000100");
UUID_DEF!(IMU_DATA_CHAR_UUID = "45e43ca9-ba7d-4769-bc47-34578c000101");
UUID_DEF!(IMU_STATUS_CHAR_UUID = "45e43ca9-ba7d-4769-bc47-34578c000102");
UUID_DEF!(IMU_CONF_CHAR_UUID = "45e43ca9-ba7d-4769-bc47-34578c00010a");
UUID_DEF!(NAVIGATION_SERVICE_UUID = "45e43ca9-ba7d-4769-bc47-34578c000200");
UUID_DEF!(GPS_DATA_CHAR_UUID = "45e43ca9-ba7d-4769-bc47-34578c000201");
UUID_DEF!(GPS_CONF_CHAR_UUID = "45e43ca9-ba7d-4769-bc47-34578c00020a");
UUID_DEF!(CONTROL_SERVICE_UUID = "45e43ca9-ba7d-4769-bc47-34578c000300");
UUID_DEF!(CONTROL_COMMAND_CHAR_UUID = "45e43ca9-ba7d-4769-bc47-34578c000301");

pub trait ToBytesLe<const N: usize> {
    fn to_bytes_le(self) -> [u8; N];
}

pub trait FromBytesLe<const N: usize>: Sized {
    fn from_le_bytes(x: &[u8; N]) -> Self;
    fn from_le_bytes_slice(x: &[u8]) -> Result<Self, TryFromSliceError> {
        Ok(Self::from_le_bytes(x.try_into()?))
    }
}

pub trait ConvertBytesLe<const N: usize> {
	type Bytes;
}

impl<const N: usize, T> ConvertBytesLe<N> for T
where
    T: Sized + ToBytesLe<N> + FromBytesLe<N>,
{
	type Bytes = [u8; N];
}

#[cfg_attr(not(target_arch = "riscv32"), derive(Debug))]
pub struct Tri {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl ToBytesLe<12> for &Tri {
    fn to_bytes_le(self) -> [u8; 12] {
        let mut res = [0u8; 12];

        res[0..4].copy_from_slice(&self.x.to_le_bytes());
        res[4..8].copy_from_slice(&self.y.to_le_bytes());
        res[8..12].copy_from_slice(&self.z.to_le_bytes());

        res
    }
}

impl FromBytesLe<12> for Tri {
    fn from_le_bytes(x: &[u8; 12]) -> Self {
        Self {
            x: f32::from_le_bytes(x[0..4].try_into().unwrap()),
            y: f32::from_le_bytes(x[4..8].try_into().unwrap()),
            z: f32::from_le_bytes(x[8..12].try_into().unwrap()),
        }
    }
}

#[cfg_attr(not(target_arch = "riscv32"), derive(Debug))]
pub struct ImuData {
    pub accel: Tri,
    pub gyro: Tri,
}

impl ToBytesLe<24> for ImuData {
    fn to_bytes_le(self) -> [u8; 24] {
        let mut data = [0; 24];

        data[0..12].copy_from_slice(&self.accel.to_bytes_le());
        data[12..24].copy_from_slice(&self.gyro.to_bytes_le());

        data
    }
}

impl FromBytesLe<24> for ImuData {
    fn from_le_bytes(x: &[u8; 24]) -> Self {
        Self {
            accel: Tri::from_le_bytes(x[0..12].try_into().unwrap()),
            gyro: Tri::from_le_bytes(x[12..24].try_into().unwrap()),
        }
    }
}

