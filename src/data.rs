use nanoserde::*;

#[derive(DeJson, Debug)]
pub struct SpriteData {
    pub x: i32,
    pub y: i32,
}

#[derive(DeJson, Debug, Clone, Copy)]
pub enum Team {
    Blue,
    Red,
}

#[derive(DeJson, Debug, Clone, Copy)]
pub enum UnitType {
    Infantry,
    Tank,
}

#[derive(DeJson, Debug)]
pub struct EntityDef {
    pub sprite: SpriteData,
    pub team: Team,
    pub unit_type: UnitType,
}

#[derive(DeJson, Debug)]
pub struct EntityOnMap {
    pub def: String,
    pub pos: [i32; 2],
}

#[derive(DeJson, Debug)]
pub struct LDTK {
    pub levels: Vec<Level>,
}

#[derive(DeJson, Debug)]
pub struct Level {
    #[nserde(rename = "layerInstances")]
    pub layers: Vec<Layer>,
    #[nserde(rename = "pxWid")]
    pub pixel_width: i32,
    #[nserde(rename = "pxHei")]
    pub pixel_height: i32,
}

#[derive(DeJson, Debug)]
pub struct Layer {
    #[nserde(rename = "__identifier")]
    pub id: String,
    //#[nserde(rename = "intGridCsv")]
    //int_grid: Vec<i64>,
    #[nserde(rename = "autoLayerTiles")]
    pub auto_tiles: Vec<AutoTile>,
}

#[derive(DeJson, Debug)]
pub struct AutoTile {
    pub px: [f32; 2],
    pub src: [i32; 2],
}
