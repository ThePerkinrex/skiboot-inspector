#![no_std]

use uuid::uuid;



pub const BASE_NAMESPACE_UUID: [u8; 16] = uuid!("45e43ca9-ba7d-4769-bc47-34578c000000").to_bytes_le();
pub const MOTION_SERVICE_UUID: [u8; 16] = uuid!("45e43ca9-ba7d-4769-bc47-34578c000100").to_bytes_le();
pub const IMU_DATA_CHAR_UUID: [u8; 16] = uuid!("45e43ca9-ba7d-4769-bc47-34578c000101").to_bytes_le();
pub const IMU_CONF_CHAR_UUID: [u8; 16] = uuid!("45e43ca9-ba7d-4769-bc47-34578c00010a").to_bytes_le();
pub const NAVIGATION_SERVICE_UUID: [u8; 16] = uuid!("45e43ca9-ba7d-4769-bc47-34578c000200").to_bytes_le();
pub const GPS_DATA_CHAR_UUID: [u8; 16] = uuid!("45e43ca9-ba7d-4769-bc47-34578c000201").to_bytes_le();
pub const GPS_CONF_CHAR_UUID: [u8; 16] = uuid!("45e43ca9-ba7d-4769-bc47-34578c00020a").to_bytes_le();
pub const CONTROL_SERVICE_UUID: [u8; 16] = uuid!("45e43ca9-ba7d-4769-bc47-34578c000300").to_bytes_le();
pub const CONTROL_COMMAND_CHAR_UUID: [u8; 16] = uuid!("45e43ca9-ba7d-4769-bc47-34578c000301").to_bytes_le();