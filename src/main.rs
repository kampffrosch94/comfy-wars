mod loading;
use comfy::*;
use grids::Grid;
use loading::*;
use nanoserde::*;

simple_game!("comfy wars", GameState, setup, update);

/// ECS marker
struct Ground;
/// ECS marker
struct Infrastructure;
/// ECS marker
struct Unit;

// constants for Z-layers
const Z_GROUND: i32 = 0;
const Z_TERRAIN: i32 = 1;
const Z_UNITS: i32 = 10;
const Z_CURSOR: i32 = 1000;

#[derive(Debug)]
pub struct GameState {
    ui: UIState,
    sprites: HashMap<String, SpriteData>,
    entity_defs: HashMap<String, EntityDef>,
    grid: Grid<i32>,
}

#[derive(Debug, Default)]
struct UIState {
    right_click_menu_pos: Option<Vec2>,
    draw_dijkstra_map: bool,
}

impl GameState {
    pub fn new(_c: &mut EngineContext) -> Self {
        Self {
            ui: Default::default(),
            sprites: Default::default(),
            entity_defs: Default::default(),
            grid: Grid::new(0, 0, 0),
        }
    }
}

const GRIDSIZE: i32 = 16;

fn setup(s: &mut GameState, c: &mut EngineContext) {
    // can be turned on by hitting F8
    c.config.borrow_mut().dev.show_fps = false;

    // load tiles
    let ldtk: LDTK = DeJson::deserialize_json(kf_include_str!("/assets/comfy_wars.ldtk")).unwrap();
    {
        let level = &ldtk.levels[0];
        let (w, h) = (level.pixel_width / GRIDSIZE, level.pixel_height / GRIDSIZE);
        s.grid = Grid::new(w, h, 0);
    }

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
    let map_entities: Vec<EntityOnMap> = DeJson::deserialize_json(ed).unwrap();

    for tile in ldtk
        .levels
        .iter()
        .flat_map(|level| level.layers.iter())
        .filter(|layer| layer.id == "groundgrid")
        .flat_map(|layer| layer.auto_tiles.iter())
    {
        c.commands().spawn((
            Sprite::new("tilemap".to_string(), vec2(1.0, 1.0), Z_GROUND, WHITE).with_rect(
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
            Sprite::new("tilemap".to_string(), vec2(1.0, 1.0), Z_TERRAIN, WHITE).with_rect(
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

    for me in map_entities {
        let def = &s.entity_defs[&me.def];
        c.commands().spawn((
            Sprite::new("tilemap".to_string(), vec2(1.0, 1.0), Z_UNITS, WHITE).with_rect(
                def.sprite.x,
                def.sprite.y,
                GRIDSIZE,
                GRIDSIZE,
            ),
            Transform::position(vec2(me.pos[0] as f32, -me.pos[1] as f32)),
            Unit,
            def.team,
            def.unit_type,
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
        s.ui.right_click_menu_pos = Some(mouse_world());
    }
    if is_mouse_button_down(MouseButton::Left) {
        s.ui.right_click_menu_pos = None;
    }

    if let Some(wpos) = s.ui.right_click_menu_pos {
        draw_cursor(s, wpos);
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
                                    Sprite::new(
                                        "tilemap".to_string(),
                                        vec2(1.0, 1.0),
                                        Z_UNITS,
                                        WHITE,
                                    )
                                    .with_rect(sprite.x, sprite.y, GRIDSIZE, GRIDSIZE),
                                ));
                            }
                        }
                    });
            });
    } else {
        draw_cursor(s, mouse_world())
    }

    egui::Window::new("kf_debug_info").show(c.egui, |ui| {
        let text = format!("mouse grid pos: {}", grid_pos(mouse_world()));
        ui.label(text);
        ui.separator();
        ui.label("Entitiy transforms:");
        for (_, (trans, ut)) in c.world().query::<(&Transform, &UnitType)>().iter() {
            ui.label(format!(
                "{:?}: {},{}",
                ut, trans.position.x, trans.position.y
            ));
        }
    });

    if is_key_pressed(KeyCode::L) {
        s.ui.draw_dijkstra_map = !s.ui.draw_dijkstra_map;
    }
    if s.ui.draw_dijkstra_map {
        for (x, y, val) in s.grid.iter() {
            let pos = vec2(x as _, -y as _);
            draw_rect(
                pos,
                vec2(1., 1.),
                Color {
                    r: 0.1,
                    g: 0.1,
                    b: 0.1,
                    a: 0.5,
                },
                50,
            );
            draw_text(&val.to_string(), pos, WHITE, TextAlign::Center);
        }
    }
}

fn draw_cursor(s: &GameState, pos: Vec2) {
    draw_sprite_ex(
        texture_id("tilemap"),
        grid_pos(pos),
        WHITE,
        Z_CURSOR,
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

#[derive(DeJson, Debug)]
struct SpriteData {
    x: i32,
    y: i32,
}

#[derive(DeJson, Debug, Clone, Copy)]
enum Team {
    Blue,
    Red,
}

#[derive(DeJson, Debug, Clone, Copy)]
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
    def: String,
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
    #[nserde(rename = "pxWid")]
    pixel_width: i32,
    #[nserde(rename = "pxHei")]
    pixel_height: i32,
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
