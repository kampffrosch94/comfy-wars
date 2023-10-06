/// stuff loaded from ldtk and co
use nanoserde::*;
use grids::Grid;
use crate::*;

#[derive(DeJson, Debug)]
pub struct SpriteData {
    pub x: i32,
    pub y: i32,
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
    #[nserde(rename = "intGridCsv")]
    int_grid: Vec<i32>,
    #[nserde(rename = "autoLayerTiles")]
    pub auto_tiles: Vec<AutoTile>,
    #[nserde(rename = "__cWid")]
    pub width: i32,
    #[nserde(rename = "__cHei")]
    pub height: i32,
}

#[derive(DeJson, Debug)]
pub struct AutoTile {
    pub px: [f32; 2],
    pub src: [i32; 2],
}

pub fn grid_from_layer<T: Clone, F: Fn(i32) -> T >(layer: &Layer, converter: F ) -> Grid<T> {
    let width = layer.width;
    let height = layer.height;
    Grid::filled_with(width, height, |x, y| {
        converter(layer.int_grid[(x + y * width) as usize] )
    })
}
