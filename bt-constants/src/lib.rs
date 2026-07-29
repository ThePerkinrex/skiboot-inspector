#![no_std]

use uuid::{Uuid, uuid};

pub const BASE_NAMESPACE_UUID: Uuid = uuid!("45e43ca9-ba7d-4769-bc47-34578c000000");
pub const MOTION_SERVICE_UUID: Uuid = uuid!("45e43ca9-ba7d-4769-bc47-34578c000100");
pub const IMU_DATA_CHAR_UUID: Uuid = uuid!("45e43ca9-ba7d-4769-bc47-34578c000101");
pub const IMU_CONF_CHAR_UUID: Uuid = uuid!("45e43ca9-ba7d-4769-bc47-34578c00010a");
pub const NAVIGATION_SERVICE_UUID: Uuid = uuid!("45e43ca9-ba7d-4769-bc47-34578c000200");
pub const GPS_DATA_CHAR_UUID: Uuid = uuid!("45e43ca9-ba7d-4769-bc47-34578c000201");
pub const GPS_CONF_CHAR_UUID: Uuid = uuid!("45e43ca9-ba7d-4769-bc47-34578c00020a");
pub const CONTROL_SERVICE_UUID: Uuid = uuid!("45e43ca9-ba7d-4769-bc47-34578c000300");
pub const CONTROL_COMMAND_CHAR_UUID: Uuid = uuid!("45e43ca9-ba7d-4769-bc47-34578c000301");