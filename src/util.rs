use crate::comfy_compat::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(remote = "Vec2")]
pub struct Vec2Proxy {
    pub x: f32,
    pub y: f32,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "IVec2")]
pub struct IVec2Proxy {
    pub x: i32,
    pub y: i32,
}
