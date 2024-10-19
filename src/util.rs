use nanoserde::*;
use comfy::*;

#[derive(Debug, DeBin, SerBin)]
pub struct Vec2Proxy {
    pub x: f32,
    pub y: f32,
}

impl From<&Vec2> for Vec2Proxy {
    fn from(value: &Vec2) -> Self {
	Self{x: value.x, y: value.y }
    }
}

impl Into<Vec2> for &Vec2Proxy {
    fn into(self) -> Vec2 {
        Vec2{x: self.x, y: self.y}
    }
}

#[derive(Debug, DeBin, SerBin)]
pub struct IVec2Proxy {
    pub x: i32,
    pub y: i32,
}

impl From<&IVec2> for IVec2Proxy {
    fn from(value: &IVec2) -> Self {
	Self{x: value.x, y: value.y }
    }
}

impl Into<IVec2> for &IVec2Proxy {
    fn into(self) -> IVec2 {
        IVec2{x: self.x, y: self.y}
    }
}
