pub use num_enum::{IntoPrimitive, FromPrimitive};

#[repr(u8)]
#[derive(Default, IntoPrimitive, FromPrimitive, Clone, Copy, Debug)]
#[non_exhaustive]
pub enum ImuCommand {
	#[default]
	None = 0,
	Calibrate
}

#[repr(u8)]
#[derive(Default, IntoPrimitive, FromPrimitive, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImuStatus {
	#[default]
	Initializing = 0,
	Ok,
	Calibrating
}