#![no_std]

pub mod imu_conf;

macro_rules! UUID_DEF {
    ($name:ident = $uuid:literal) => {
        #[cfg(target_arch = "riscv32")]
        pub const $name: [u8; 16] = {
            let mut a = uuid::uuid!($uuid).into_bytes();
            a.reverse();
            a
        };
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

pub trait ByteSize {
    const SIZE: usize;
}

pub trait ToBytesLe: ByteSize {
    /// Writes `Self::SIZE` bytes into the front of `buf` and returns the
    /// number of bytes written. `buf` may be longer than `Self::SIZE`.
    fn to_bytes_le(self, buf: &mut [u8]) -> usize;
}

pub trait FromBytesLe: Sized + ByteSize {
    /// Reads `Self` from the front of `x`. `x` must be at least `Self::SIZE`
    /// bytes long, or this will panic on an out-of-bounds slice index.
    fn from_le_bytes(x: &[u8]) -> Self;

    /// Bounds-checked variant: returns `None` if `x` is too short.
    fn try_from_le_bytes(x: &[u8]) -> Option<Self> {
        if x.len() < Self::SIZE {
            None
        } else {
            Some(Self::from_le_bytes(x))
        }
    }
}

impl ByteSize for f32 {
    const SIZE: usize = 4;
}

impl ToBytesLe for f32 {
    fn to_bytes_le(self, buf: &mut [u8]) -> usize {
        buf[0..4].copy_from_slice(&self.to_le_bytes());
        4
    }
}

impl FromBytesLe for f32 {
    fn from_le_bytes(x: &[u8]) -> Self {
        Self::from_le_bytes(x[0..4].try_into().unwrap())
    }
}

impl ByteSize for i16 {
    const SIZE: usize = 2;
}

impl ToBytesLe for i16 {
    fn to_bytes_le(self, buf: &mut [u8]) -> usize {
        buf[0..2].copy_from_slice(&self.to_le_bytes());
        2
    }
}

impl FromBytesLe for i16 {
    fn from_le_bytes(x: &[u8]) -> Self {
        Self::from_le_bytes(x[0..2].try_into().unwrap())
    }
}

// Mirror: array size lives on ByteSize alone, so it needs its own impl,
// separate from the ToBytesLe/FromBytesLe array impls below.
impl<T, const N: usize> ByteSize for [T; N]
where
    T: ByteSize,
{
    const SIZE: usize = N * T::SIZE;
}

impl<T, const N: usize> ToBytesLe for [T; N]
where
    T: ToBytesLe,
{
    fn to_bytes_le(self, buf: &mut [u8]) -> usize {
        let mut offset = 0;
        for item in self {
            offset += item.to_bytes_le(&mut buf[offset..]);
        }
        offset
    }
}

impl<T, const N: usize> FromBytesLe for [T; N]
where
    T: FromBytesLe,
{
    fn from_le_bytes(x: &[u8]) -> Self {
        core::array::from_fn(|i| T::from_le_bytes(&x[i * T::SIZE..(i + 1) * T::SIZE]))
    }
}

#[cfg_attr(not(target_arch = "riscv32"), derive(Debug))]
pub struct Tri {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl ByteSize for Tri {
    const SIZE: usize = 12;
}

impl ToBytesLe for Tri {
    fn to_bytes_le(self, buf: &mut [u8]) -> usize {
        let mut offset = 0;
        offset += self.x.to_bytes_le(&mut buf[offset..]);
        offset += self.y.to_bytes_le(&mut buf[offset..]);
        offset += self.z.to_bytes_le(&mut buf[offset..]);
        offset
    }
}

impl FromBytesLe for Tri {
    fn from_le_bytes(x: &[u8]) -> Self {
        Self {
            x: <f32 as FromBytesLe>::from_le_bytes(&x[0..4]),
            y: <f32 as FromBytesLe>::from_le_bytes(&x[4..8]),
            z: <f32 as FromBytesLe>::from_le_bytes(&x[8..12]),
        }
    }
}

#[cfg_attr(not(target_arch = "riscv32"), derive(Debug))]
pub struct ImuData {
    pub accel: Tri,
    pub gyro: Tri,
    pub quat: [f32; 4],
}

impl ByteSize for ImuData {
    const SIZE: usize = Tri::SIZE + Tri::SIZE + <[f32; 4] as ByteSize>::SIZE; // 40
}

impl ToBytesLe for ImuData {
    fn to_bytes_le(self, buf: &mut [u8]) -> usize {
        let mut offset = 0;
        offset += self.accel.to_bytes_le(&mut buf[offset..]);
        offset += self.gyro.to_bytes_le(&mut buf[offset..]);
        offset += self.quat.to_bytes_le(&mut buf[offset..]);
        offset
    }
}

impl FromBytesLe for ImuData {
    fn from_le_bytes(x: &[u8]) -> Self {
        Self {
            accel: Tri::from_le_bytes(&x[0..12]),
            gyro: Tri::from_le_bytes(&x[12..24]),
            quat: <[f32; 4]>::from_le_bytes(&x[24..40]),
        }
    }
}