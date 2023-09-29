mod loading;
use comfy::*;
use loading::*;
use nanoserde::*;

simple_game!("comfy wars", GameState, setup, update);

// ECS markers
struct Ground;
struct Infrastructure;
struct Unit;

#[derive(Debug, Default)]
pub struct GameState {
    right_click_menu_pos: Option<Vec2>,
    sprites: HashMap<String, SpriteData>,
    entity_defs: HashMap<String, EntityDef>,
}

impl GameState {
    pub fn new(_c: &mut EngineContext) -> Self {
        Self::default()
    }
}

const GRIDSIZE: i32 = 16;

fn setup(s: &mut GameState, c: &mut EngineContext) {
    // can be turned on by hitting F8
    c.config.borrow_mut().dev.show_fps = false;

    // load tiles
    let ldtk: LDTK = DeJson::deserialize_json(kf_include_str!("/assets/comfy_wars.ldtk")).unwrap();

    c.load_texture_from_bytes(
        "tilemap",
        kf_include_bytes!("/assets/tilemap/tilemap_packed.png"),
    );

    // load sprites
    let sprites_str = kf_include_str!("/assets/sprites.json");
    s.sprites = DeJson::deserialize_json(sprites_str).unwrap();

    // load entity definitions
    let ed = kf_include_str!("/assets/entities_def.json");
    s.entity_defs = DeJson::deserialize_json(ed).unwrap();

    // load entities on map
    let ed = kf_include_str!("/assets/entities_map.json");
    let map_entities: HashMap<String, EntityOnMap> = DeJson::deserialize_json(ed).unwrap();
    dbg!(&map_entities);


    for tile in ldtk
        .levels
        .iter()
        .flat_map(|level| level.layers.iter())
        .filter(|layer| layer.id == "groundgrid")
        .flat_map(|layer| layer.auto_tiles.iter())
    {
        c.commands().spawn((
            Sprite::new("tilemap".to_string(), vec2(1.0, 1.0), 0, WHITE).with_rect(
                tile.src[0],
                tile.src[1],
                GRIDSIZE,
                GRIDSIZE,
            ),
            Transform::position(vec2(
                tile.px[0] / GRIDSIZE as f32,
                -tile.px[1] / GRIDSIZE as f32,
            )),
            Ground,
        ));
    }

    for tile in ldtk
        .levels
        .iter()
        .flat_map(|level| level.layers.iter())
        .filter(|layer| layer.id == "infrastructuregrid")
        .flat_map(|layer| layer.auto_tiles.iter())
    {
        c.commands().spawn((
            Sprite::new("tilemap".to_string(), vec2(1.0, 1.0), 1, WHITE).with_rect(
                tile.src[0],
                tile.src[1],
                GRIDSIZE,
                GRIDSIZE,
            ),
            Transform::position(vec2(
                tile.px[0] / GRIDSIZE as f32,
                -tile.px[1] / GRIDSIZE as f32,
            )),
            Infrastructure,
        ));
    }
}

fn update(s: &mut GameState, c: &mut EngineContext) {
    span_with_timing!("kf/update");
    clear_background(TEAL);
    let mut visuals = egui::Visuals::dark();
    visuals.window_shadow = epaint::Shadow {
        extrusion: 0.,
        color: epaint::Color32::BLACK,
    };
    c.egui.set_visuals(visuals);

    let c_x = tweak!(6.);
    let c_y = tweak!(-7.);
    main_camera_mut().center = Vec2::new(c_x, c_y);

    if is_mouse_button_down(MouseButton::Right) {
        s.right_click_menu_pos = Some(mouse_world());
    }
    if is_mouse_button_down(MouseButton::Left) {
        s.right_click_menu_pos = None;
    }

    if let Some(wpos) = s.right_click_menu_pos {
        let pos = world_to_screen(wpos);
        egui::Area::new("context_menu")
            .fixed_pos(egui::pos2(pos.x, pos.y))
            .show(c.egui, |ui| {
                egui::Frame::none()
                    .fill(egui::Color32::BLACK)
                    .show(ui, |ui| {
                        for (name, sprite) in s.sprites.iter().sorted_by_key(|s| s.0) {
                            if ui.button(name).clicked() {
                                c.commands().spawn((
                                    Unit,
                                    Transform::position(grid_pos(wpos)),
                                    Sprite::new("tilemap".to_string(), vec2(1.0, 1.0), 10, WHITE)
                                        .with_rect(sprite.x, sprite.y, GRIDSIZE, GRIDSIZE),
                                ));
                            }
                        }
                    });
            });
    }
    draw_sprite_ex(
        texture_id("tilemap"),
        mouse_grid(),
        WHITE,
        20,
        DrawTextureParams {
            dest_size: Some(vec2(1.0, 1.0).as_world_size()),
            source_rect: Some(IRect {
                offset: ivec2(s.sprites["cursor"].x, s.sprites["cursor"].y),
                size: ivec2(GRIDSIZE, GRIDSIZE),
            }),
            ..Default::default()
        },
    );
}

fn grid_pos(v: Vec2) -> Vec2 {
    Vec2 {
        x: v.x.round(),
        y: v.y.round(),
    }
}

fn mouse_grid() -> Vec2 {
    grid_pos(mouse_world())
}

#[derive(DeJson, Debug)]
struct SpriteData {
    x: i32,
    y: i32,
}


#[derive(DeJson, Debug)]
enum Team {
    Blue,
    Red,
}

#[derive(DeJson, Debug)]
enum UnitType {
    Infantry,
    Tank,
}

#[derive(DeJson, Debug)]
struct EntityDef {
    sprite: SpriteData,
    team: Team,
    unit_type: UnitType,
}

#[derive(DeJson, Debug)]
struct EntityOnMap {
    pos: [i32; 2],
}

#[derive(DeJson, Debug)]
struct LDTK {
    levels: Vec<Level>,
}

#[derive(DeJson, Debug)]
struct Level {
    #[nserde(rename = "layerInstances")]
    layers: Vec<Layer>,
}

#[derive(DeJson, Debug)]
struct Layer {
    #[nserde(rename = "__identifier")]
    id: String,
    //#[nserde(rename = "intGridCsv")]
    //int_grid: Vec<i64>,
    #[nserde(rename = "autoLayerTiles")]
    auto_tiles: Vec<AutoTile>,
}

#[derive(DeJson, Debug)]
struct AutoTile {
    px: [f32; 2],
    src: [i32; 2],
}
