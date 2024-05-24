// For now, this only has information about the firmware,
// not the firmware itself.

// Keep these in sync with arduino code.

pub const SENSOR_CHARACTERISTICS_UUID: &str = "0a3d3fd8-2f1c-46fd-bf46-eaef2fda91e5";
pub const CHANNEL_COUNT_CHARACTERISTICS_UUID: &str = "0a3d3fd8-2f1c-46fd-bf46-eaef2fda91e6";

pub const SAMPLE_DELAY_PARAM_A: f64 = -11.3384217;
pub const SAMPLE_DELAY_PARAM_B: f64 = 1.93093431;
pub const PROTOCOL_HEADER_LEN: i32 = 8;
