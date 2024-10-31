use crate::comfy_compat::*;
use derive_more::{derive::MulAssign, Add, AddAssign, Div, DivAssign, From, Mul, Sub, SubAssign};
use macroquad::prelude::Vec2;
use serde::{Deserialize, Serialize};
use tween::TweenValue;

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

// needed because orphan rules are annoying
#[derive(
    Default,
    Clone,
    Copy,
    Debug,
    Add,
    Sub,
    Mul,
    Div,
    From,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    Deserialize,
    Serialize,
)]
pub struct Vec2f {
    pub x: f32,
    pub y: f32,
}

impl TweenValue for Vec2f {
    fn scale(self, scale: f32) -> Self {
        self * scale
    }
}

impl From<Vec2> for Vec2f {
    fn from(value: Vec2) -> Self {
        Vec2f {
            x: value.x,
            y: value.y,
        }
    }
}

impl Into<Vec2> for Vec2f {
    fn into(self) -> Vec2 {
        Vec2 {
            x: self.x,
            y: self.y,
        }
    }
}
