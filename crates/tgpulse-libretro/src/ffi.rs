//! Minimal Libretro ABI declarations used by this adapter. Values and layouts
//! follow `libretro.h`; keep this list narrow as the adapter gains features.

use std::ffi::{c_char, c_void};

pub type Environment = unsafe extern "C" fn(u32, *mut c_void) -> bool;
pub type Video = unsafe extern "C" fn(*const c_void, u32, u32, usize);
pub type AudioSample = unsafe extern "C" fn(i16, i16);
pub type AudioBatch = unsafe extern "C" fn(*const i16, usize) -> usize;
pub type InputPoll = unsafe extern "C" fn();
pub type InputState = unsafe extern "C" fn(u32, u32, u32, u32) -> i16;
pub type SetRumbleState = unsafe extern "C" fn(u32, u32, u16) -> bool;

pub const API_VERSION: u32 = 1;
pub const MEMORY_SAVE_RAM: u32 = 0;
pub const SET_MESSAGE: u32 = 6;
pub const GET_MESSAGE_INTERFACE_VERSION: u32 = 59;
pub const SET_MESSAGE_EXT: u32 = 60;
pub const SHUTDOWN: u32 = 7;
pub const SET_PIXEL_FORMAT: u32 = 10;
pub const SET_INPUT_DESCRIPTORS: u32 = 11;
pub const SET_CONTROLLER_INFO: u32 = 35;
pub const SET_GEOMETRY: u32 = 37;
pub const GET_RUMBLE_INTERFACE: u32 = 23;
pub const RUMBLE_STRONG: u32 = 0;
pub const RUMBLE_WEAK: u32 = 1;
pub const GET_VARIABLE: u32 = 15;
pub const GET_SYSTEM_DIRECTORY: u32 = 9;
pub const GET_SAVE_DIRECTORY: u32 = 31;
pub const SET_VARIABLES: u32 = 16;
pub const GET_VARIABLE_UPDATE: u32 = 17;
pub const GET_CORE_OPTIONS_VERSION: u32 = 52;
pub const SET_CORE_OPTIONS_V2: u32 = 67;
pub const SET_CORE_OPTIONS_DISPLAY: u32 = 55;
pub const SET_CORE_OPTIONS_UPDATE_DISPLAY_CALLBACK: u32 = 69;
pub const PIXEL_FORMAT_XRGB8888: u32 = 1;
pub const REGION_NTSC: u32 = 0;
pub const DEVICE_JOYPAD: u32 = 1;
pub const DEVICE_NONE: u32 = 0;
pub const DEVICE_ANALOG: u32 = 5;
pub const ANALOG_LEFT: u32 = 0;
pub const ANALOG_RIGHT: u32 = 1;
pub const ANALOG_BUTTON: u32 = 2;
pub const ANALOG_X: u32 = 0;
pub const ANALOG_Y: u32 = 1;
pub const JOY_B: u32 = 0;
pub const JOY_Y: u32 = 1;
pub const JOY_SELECT: u32 = 2;
pub const JOY_START: u32 = 3;
pub const JOY_UP: u32 = 4;
pub const JOY_DOWN: u32 = 5;
pub const JOY_LEFT: u32 = 6;
pub const JOY_RIGHT: u32 = 7;
pub const JOY_A: u32 = 8;
pub const JOY_X: u32 = 9;
pub const JOY_L: u32 = 10;
pub const JOY_R: u32 = 11;
pub const JOY_L2: u32 = 12;
pub const JOY_R2: u32 = 13;
pub const JOY_L3: u32 = 14;
pub const JOY_R3: u32 = 15;

#[repr(C)]
pub struct SystemInfo {
    pub library_name: *const c_char,
    pub library_version: *const c_char,
    pub valid_extensions: *const c_char,
    pub need_fullpath: bool,
    pub block_extract: bool,
}

#[repr(C)]
pub struct Geometry {
    pub base_width: u32,
    pub base_height: u32,
    pub max_width: u32,
    pub max_height: u32,
    pub aspect_ratio: f32,
}

#[repr(C)]
pub struct Timing {
    pub fps: f64,
    pub sample_rate: f64,
}

#[repr(C)]
pub struct AvInfo {
    pub geometry: Geometry,
    pub timing: Timing,
}

#[repr(C)]
pub struct GameInfo {
    pub path: *const c_char,
    pub data: *const c_void,
    pub size: usize,
    pub meta: *const c_char,
}

#[repr(C)]
pub struct Message {
    pub msg: *const c_char,
    pub frames: u32,
}

#[repr(C)]
pub struct MessageExt {
    pub msg: *const c_char,
    pub duration: u32,
    pub priority: u32,
    pub level: u32,
    pub target: u32,
    pub message_type: u32,
    pub progress: i8,
}

#[repr(C)]
pub struct OptionDisplay {
    pub key: *const c_char,
    pub visible: bool,
}

#[repr(C)]
pub struct OptionUpdateDisplay {
    pub callback: Option<unsafe extern "C" fn() -> bool>,
}

#[repr(C)]
pub struct RumbleInterface {
    pub set_rumble_state: Option<SetRumbleState>,
}

#[repr(C)]
pub struct InputDescriptor {
    pub port: u32,
    pub device: u32,
    pub index: u32,
    pub id: u32,
    pub description: *const c_char,
}

#[repr(C)]
pub struct ControllerDescription {
    pub desc: *const c_char,
    pub id: u32,
}

#[repr(C)]
pub struct ControllerInfo {
    pub types: *const ControllerDescription,
    pub num_types: u32,
}

#[repr(C)]
pub struct Variable {
    pub key: *const c_char,
    pub value: *const c_char,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct OptionValue {
    pub value: *const c_char,
    pub label: *const c_char,
}

#[repr(C)]
pub struct OptionCategory {
    pub key: *const c_char,
    pub desc: *const c_char,
    pub info: *const c_char,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct OptionDefinition {
    pub key: *const c_char,
    pub desc: *const c_char,
    pub desc_categorized: *const c_char,
    pub info: *const c_char,
    pub info_categorized: *const c_char,
    pub category_key: *const c_char,
    pub values: [OptionValue; 128],
    pub default_value: *const c_char,
}

#[repr(C)]
pub struct OptionsV2 {
    pub categories: *const OptionCategory,
    pub definitions: *const OptionDefinition,
}

// Official frontend-owned Netpacket API (libretro.h environment 78).
pub const SET_NETPACKET_INTERFACE: u32 = 78;
pub type NetSend = unsafe extern "C" fn(i32, *const c_void, usize, u16);
pub type NetPoll = unsafe extern "C" fn();
#[repr(C)]
pub struct NetCallbacks {
    pub start: unsafe extern "C" fn(u16, Option<NetSend>, Option<NetPoll>),
    pub receive: unsafe extern "C" fn(*const c_void, usize, u16),
    pub stop: Option<unsafe extern "C" fn()>,
    pub poll: Option<NetPoll>,
    pub connected: Option<unsafe extern "C" fn(u16) -> bool>,
    pub disconnected: Option<unsafe extern "C" fn(u16)>,
    pub protocol_version: *const c_char,
}

// Experimental sensor extension: ABI and units follow libretro.h.
pub const GET_SENSOR_INTERFACE: u32 = 25 | 0x10000;
pub type SetSensorState = unsafe extern "C" fn(u32, u32, u32) -> bool;
pub type GetSensorInput = unsafe extern "C" fn(u32, u32) -> f32;
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct SensorInterface {
    pub set_sensor_state: Option<SetSensorState>,
    pub get_sensor_input: Option<GetSensorInput>,
}
