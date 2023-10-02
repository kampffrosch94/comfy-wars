mod data;
mod dijkstra;
mod loading;

use comfy::*;
use cosync::{Cosync, CosyncQueueHandle};
use data::*;
use dijkstra::*;
use grids::Grid;
use loading::*;
use nanoserde::*;

simple_game!("comfy wars", GameWrapper, setup, update);

/// ECS marker
struct Ground;
/// ECS marker
struct Infrastructure;

// constants for Z-layers
const Z_GROUND: i32 = 0;
const Z_TERRAIN: i32 = 10;
const Z_MOVE_HIGHLIGHT: i32 = 11;
const Z_MOVE_ARROW: i32 = 12;
const Z_UNITS: i32 = 20;
const Z_CURSOR: i32 = 1000;

pub struct GameWrapper {
    cosync: Cosync<GameState>,
    game_state: GameState,
}

impl GameWrapper {
    pub fn new(c: &mut EngineContext) -> Self {
        let cosync = Cosync::new();
        let handle = cosync.create_queue_handle();
        Self {
            cosync,
            game_state: GameState::new(c, handle),
        }
    }
}

pub struct GameState {
    co: CosyncQueueHandle<GameState>,
    ui: UIState,
    sprites: HashMap<String, SpriteData>,
    entity_defs: HashMap<String, EntityDef>,
    grids: Grids,
    entities: Arena<Actor>,
}

impl GameState {
    pub fn new(_c: &mut EngineContext, co: CosyncQueueHandle<GameState>) -> Self {
        Self {
            ui: Default::default(),
            sprites: Default::default(),
            entity_defs: Default::default(),
            grids: Default::default(),
            co,
            entities: Arena::new(),
        }
    }
}

#[derive(Debug, Default)]
struct UIState {
    right_click_menu_pos: Option<Vec2>,
    draw_dijkstra_map: bool,
    selected_entity: Option<Index>,
}

/// all grids in here have the same dimensions
/// bigger x is right
/// bigger y is down (reverse of what comfy uses atm)
#[derive(Debug)]
struct Grids {
    dijkstra: Grid<i32>,
    ground: Grid<GroundType>,
    terrain: Grid<TerrainType>,
}

impl Default for Grids {
    fn default() -> Self {
        Self {
            dijkstra: Grid::new(0, 0, 0),
            ground: Grid::new(0, 0, Default::default()),
            terrain: Grid::new(0, 0, Default::default()),
        }
    }
}

/// used for determining movement cost
#[derive(Debug, Default, Clone, Copy)]
enum GroundType {
    #[default]
    Ground,
    Water,
}

/// used for determining movement cost
#[derive(Debug, Default, Clone, Copy)]
enum TerrainType {
    #[default]
    None,
    Street,
    Forest,
}

#[derive(Debug)]
struct Actor {
    pos: IVec2,
    draw_pos: Vec2,
    sprite_coords: IVec2,
    team: Team,
    unit_type: UnitType,
}

const GRIDSIZE: i32 = 16;

fn setup(s: &mut GameWrapper, c: &mut EngineContext) {
    let s = &mut s.game_state;
    // load tiles
    let ldtk: LDTK = DeJson::deserialize_json(kf_include_str!("/assets/comfy_wars.ldtk")).unwrap();
    {
        let level = &ldtk.levels[0];
        let (w, h) = (level.pixel_width / GRIDSIZE, level.pixel_height / GRIDSIZE);
        s.grids.dijkstra = Grid::new(w, h, 0);
        s.grids.ground = Grid::new(w, h, Default::default());
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

    for layer in ldtk
        .levels
        .iter()
        .flat_map(|level| level.layers.iter())
        .filter(|layer| layer.id == "groundgrid")
    {
        s.grids.ground = grid_from_layer(layer, |i| match i {
            1 => GroundType::Ground,
            2 => GroundType::Water,
            _ => panic!("unsupported ground type {}", i),
        });
        for tile in layer.auto_tiles.iter() {
            commands().spawn((
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
    }

    for layer in ldtk
        .levels
        .iter()
        .flat_map(|level| level.layers.iter())
        .filter(|layer| layer.id == "infrastructuregrid")
    {
        for tile in layer.auto_tiles.iter() {
            s.grids.terrain = grid_from_layer(layer, |i| match i {
                0 => TerrainType::None,
                1 | 2 | 3 | 4 => TerrainType::Street,
                5 => TerrainType::Forest,
                _ => panic!("unsupported terrain type {}", i),
            });
            commands().spawn((
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
    }

    for me in map_entities {
        let def = &s.entity_defs[&me.def];
        s.entities.insert(Actor {
            pos: me.pos.into(),
            draw_pos: vec2(me.pos[0] as f32, -me.pos[1] as f32),
            sprite_coords: ivec2(def.sprite.x, def.sprite.y),
            team: def.team,
            unit_type: def.unit_type,
        });
    }
}

fn update(s: &mut GameWrapper, _c: &mut EngineContext) {
    span_with_timing!("kf/update");
    let co = &mut s.cosync;
    let s = &mut s.game_state;
    co.run_until_stall(s);
    clear_background(TEAL);
    let mut visuals = egui::Visuals::dark();
    visuals.window_shadow = epaint::Shadow {
        extrusion: 0.,
        color: epaint::Color32::BLACK,
    };
    egui().set_visuals(visuals);

    let c_x = tweak!(6.);
    let c_y = tweak!(-7.);
    main_camera_mut().center = Vec2::new(c_x, c_y);

    handle_input(s);
    handle_debug_input(s);
}

/// relevant for the actual game
/// also does drawing in immediate mode
fn handle_input(s: &mut GameState) {
    if is_mouse_button_released(MouseButton::Right) {
        s.ui.right_click_menu_pos = Some(mouse_world());
    }
    if is_mouse_button_released(MouseButton::Left) {
        s.ui.right_click_menu_pos = None;
        let pos = grid_world_pos(mouse_world());
        s.ui.selected_entity = None;

        for (key, actor) in s.entities.iter() {
            // I am scared of floats
            if pos.abs_diff_eq(actor.draw_pos, 0.01) {
                s.ui.selected_entity = Some(key);
            }
        }
    }

    if let Some(wpos) = s.ui.right_click_menu_pos {
        draw_cursor(s, wpos);
        let pos = world_to_screen(wpos);
        egui::Area::new("context_menu")
            .fixed_pos(egui::pos2(pos.x, pos.y))
            .show(egui(), |ui| {
                egui::Frame::none()
                    .fill(egui::Color32::BLACK)
                    .show(ui, |ui| {
                        for (name, sprite) in s.sprites.iter().sorted_by_key(|s| s.0) {
                            if ui.button(name).clicked() {
                                commands().spawn((
                                    Transform::position(grid_world_pos(wpos)),
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
    } else if let Some(e) = s.ui.selected_entity {
        let pos = s.entities[e].draw_pos;
        draw_cursor(s, pos);

        let grid = &mut s.grids.dijkstra;
        let gp = grid_pos(pos);
        grid.iter_values_mut().for_each(|val| *val = 0);
        grid[gp] = 9;
        let cost = |v| -> i32 {
            let ground = *s.grids.ground.get_clamped_v(v);
            let terrain = *s.grids.terrain.get_clamped_v(v);
            match ground {
                GroundType::Water => 9999,
                GroundType::Ground => match terrain {
                    TerrainType::None => 2,
                    TerrainType::Street => 1,
                    TerrainType::Forest => 3,
                },
            }
        };
        dijkstra(grid, &[gp], cost);
        let map = &s.grids.dijkstra;
        draw_move_range(s, map);
        let path = draw_move_path(s, map, mouse_game_grid());
        if is_mouse_button_pressed(MouseButton::Left) && path.len() > 0 {
            s.co.queue(move |mut s| async move {
                for pos in path.iter().rev() {
                    let target = game_to_world(*pos);
                    let mut lerpiness = 0.;
                    while lerpiness < 1. {
                        lerpiness += delta() * 25.;
                        {
                            let s = &mut s.get();
                            let drawpos = &mut s.entities[e].draw_pos;
                            *drawpos = drawpos.lerp(target, lerpiness);
                        }
                        cosync::sleep_ticks(1).await;
                    }
                }
                let target = game_to_world(path[0]);
                {
                    let s = &mut s.get();
                    let drawpos = &mut s.entities[e].draw_pos;
                    *drawpos = target;
                }
            });
        }
    } else {
        draw_cursor(s, mouse_world())
    }

    // draw actors
    for (_index, actor) in s.entities.iter() {
        draw_sprite_ex(
            texture_id("tilemap"),
            actor.draw_pos,
            WHITE,
            Z_UNITS,
            DrawTextureParams {
                dest_size: Some(vec2(1.0, 1.0).as_world_size()),
                source_rect: Some(IRect {
                    offset: actor.sprite_coords,
                    size: ivec2(GRIDSIZE, GRIDSIZE),
                }),
                ..Default::default()
            },
        );
    }
}

/// debug information and keybindings
/// also does drawing in immediate mode
fn handle_debug_input(s: &mut GameState) {
    egui::Window::new("kf_debug_info").show(egui(), |ui| {
        let pos = grid_world_pos(mouse_world());
        ui.label(format!("mouse world grid pos: {}", pos));
        let pos = ivec2(pos.x as _, -pos.y as _);
        ui.label(format!(
            "ground type {:?}",
            s.grids.ground.get_clamped_v(pos)
        ));
        ui.label(format!(
            "terrain type {:?}",
            s.grids.terrain.get_clamped_v(pos)
        ));

        ui.separator();
        ui.label("selected Entity:");
        if let Some(e) = s.ui.selected_entity {
            let pos = s.entities[e].pos;
            ui.label(format!("position {:?}", pos));
        } else {
            ui.label("None");
        }

        ui.separator();
        ui.label("Entitiy transforms:");
        for (_index, actor) in s.entities.iter() {
            ui.label(format!(
                "{:?}: {},{}",
                actor.unit_type, actor.draw_pos.x, actor.draw_pos.y
            ));
        }
    });

    if is_key_pressed(KeyCode::L) {
        s.ui.draw_dijkstra_map = !s.ui.draw_dijkstra_map;
    }

    if s.ui.draw_dijkstra_map {
        for (x, y, val) in s.grids.dijkstra.iter() {
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
            if *val > 0 {
                draw_text(&val.to_string(), pos, WHITE, TextAlign::Center);
            }
        }
    }
}

fn draw_cursor(s: &GameState, pos: Vec2) {
    cw_draw_sprite(s, "cursor", grid_world_pos(pos), Z_CURSOR);
}

/// comfy wars specific helper for drawing sprites
fn cw_draw_sprite(s: &GameState, name: &str, pos: Vec2, z: i32) {
    draw_sprite_ex(
        texture_id("tilemap"),
        pos,
        WHITE,
        z,
        DrawTextureParams {
            dest_size: Some(vec2(1.0, 1.0).as_world_size()),
            source_rect: Some(IRect {
                offset: ivec2(s.sprites[name].x, s.sprites[name].y),
                size: ivec2(GRIDSIZE, GRIDSIZE),
            }),
            ..Default::default()
        },
    );
}

fn draw_move_range(s: &GameState, grid: &Grid<i32>) {
    for (x, y, v) in grid.iter() {
        if *v > 0 {
            let mut pos = ivec2(x, y).as_vec2();
            pos.y *= -1.;
            cw_draw_sprite(s, "move_range", pos, Z_MOVE_HIGHLIGHT);
        }
    }
}

fn draw_move_path(s: &GameState, grid: &Grid<i32>, gp: IVec2) -> Vec<IVec2> {
    const DOWN: (i32, i32) = (0, 1);
    const UP: (i32, i32) = (0, -1);
    const RIGHT: (i32, i32) = (1, 0);
    const LEFT: (i32, i32) = (-1, 0);

    let path = dijkstra_path(grid, gp);
    let mut iter = path.iter().rev();
    let prev = iter.next().cloned();
    let mut prev_direction: Option<(i32, i32)> = None;
    if let Some(mut prev) = prev {
        for pos in iter {
            let direction = (*pos - prev).into();
            if let Some(prev_direction) = prev_direction {
                let sprite = match (prev_direction, direction) {
                    (LEFT, LEFT) | (RIGHT, RIGHT) => "arrow_we",
                    (UP, UP) | (DOWN, DOWN) => "arrow_ns",
                    (DOWN, RIGHT) | (LEFT, UP) => "arrow_ne",
                    (UP, RIGHT) | (LEFT, DOWN) => "arrow_se",
                    (DOWN, LEFT) | (RIGHT, UP) => "arrow_wn",
                    (UP, LEFT) | (RIGHT, DOWN) => "arrow_ws",
                    _ => panic!("should be impossible"),
                };
                cw_draw_sprite(s, sprite, game_to_world(prev), Z_MOVE_ARROW);
            }
            prev = *pos;
            prev_direction = Some(direction);
        }
    }

    // draw ending arrow
    let len = path.len();
    if len >= 2 {
        let prev = path[1];
        let pos = path[0];
        let direction: (i32, i32) = (pos - prev).into();
        let sprite = match direction {
            LEFT => "arrow_w",
            RIGHT => "arrow_e",
            DOWN => "arrow_s",
            UP => "arrow_n",
            _ => panic!("should be impossible"),
        };
        cw_draw_sprite(s, sprite, game_to_world(pos), Z_MOVE_ARROW);
    }
    return path;
}

fn grid_world_pos(v: Vec2) -> Vec2 {
    Vec2 {
        x: v.x.round(),
        y: v.y.round(),
    }
}

fn grid_pos(v: Vec2) -> IVec2 {
    let mut r = grid_world_pos(v).as_ivec2();
    r.y *= -1;
    r
}

fn word_to_game(v: Vec2) -> IVec2 {
    let v = grid_world_pos(v);
    ivec2(v.x as _, -v.y as _)
}

fn game_to_world(v: IVec2) -> Vec2 {
    vec2(v.x as _, -v.y as _)
}

fn mouse_game_grid() -> IVec2 {
    word_to_game(mouse_world())
}
