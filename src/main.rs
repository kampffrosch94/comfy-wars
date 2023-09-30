mod data;
mod loading;
use comfy::*;
use data::*;
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
    ground_grid: Grid<GroundType>,
    terrain_grid: Grid<TerrainType>,
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
            ground_grid: Grid::new(0, 0, Default::default()),
            terrain_grid: Grid::new(0, 0, Default::default()),
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

const GRIDSIZE: i32 = 16;

fn setup(s: &mut GameState, c: &mut EngineContext) {
    // load tiles
    let ldtk: LDTK = DeJson::deserialize_json(kf_include_str!("/assets/comfy_wars.ldtk")).unwrap();
    {
        let level = &ldtk.levels[0];
        let (w, h) = (level.pixel_width / GRIDSIZE, level.pixel_height / GRIDSIZE);
        s.grid = Grid::new(w, h, 0);
        s.ground_grid = Grid::new(w, h, Default::default());
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
        s.ground_grid = grid_from_layer(layer, |i| match i {
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
            s.terrain_grid = grid_from_layer(layer, |i| match i {
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
        commands().spawn((
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
    egui().set_visuals(visuals);

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
            .show(egui(), |ui| {
                egui::Frame::none()
                    .fill(egui::Color32::BLACK)
                    .show(ui, |ui| {
                        for (name, sprite) in s.sprites.iter().sorted_by_key(|s| s.0) {
                            if ui.button(name).clicked() {
                                commands().spawn((
                                    Unit,
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
    } else {
        draw_cursor(s, mouse_world())
    }

    egui::Window::new("kf_debug_info").show(egui(), |ui| {
        let pos = grid_world_pos(mouse_world());
        ui.label(format!("mouse world grid pos: {}", pos));
        let pos = ivec2(pos.x as _, -pos.y as _);
        ui.label(format!(
            "ground type {:?}",
            s.ground_grid.get_clamped_v(pos)
        ));
        ui.label(format!(
            "terrain type {:?}",
            s.terrain_grid.get_clamped_v(pos)
        ));

        ui.separator();
        ui.label("Entitiy transforms:");
        for (_, (trans, ut)) in world().query::<(&Transform, &UnitType)>().iter() {
            ui.label(format!(
                "{:?}: {},{}",
                ut, trans.position.x, trans.position.y
            ));
        }
    });

    if is_key_pressed(KeyCode::L) {
        s.ui.draw_dijkstra_map = !s.ui.draw_dijkstra_map;
    }

    {
        s.grid.iter_values_mut().for_each(|v| *v = 0);
        let mg = grid_world_pos(mouse_world());
        let pos = ivec2(mg.x as _, -mg.y as _);
        *s.grid.get_clamped_mut(pos.x, pos.y) = 9;
        dijkstra(&mut s.grid, &[pos], |v| -> i32 {
            let ground = *s.ground_grid.get_clamped_v(v);
            let terrain = *s.terrain_grid.get_clamped_v(v);
            match ground {
                GroundType::Water => 9999,
                GroundType::Ground => match terrain {
                    TerrainType::None => 2,
                    TerrainType::Street => 1,
                    TerrainType::Forest => 3,
                },
            }
        });
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
            if *val > 0 {
                draw_text(&val.to_string(), pos, WHITE, TextAlign::Center);
            }
        }
    }
}

fn draw_cursor(s: &GameState, pos: Vec2) {
    draw_sprite_ex(
        texture_id("tilemap"),
        grid_world_pos(pos),
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

fn grid_world_pos(v: Vec2) -> Vec2 {
    Vec2 {
        x: v.x.round(),
        y: v.y.round(),
    }
}

fn get_neighbors(pos: IVec2, grid: &Grid<i32>) -> Vec<IVec2> {
    let x = pos.x;
    let y = pos.y;
    [(x - 1, y), (x + 1, y), (x, y + 1), (x, y - 1)]
        .into_iter()
        .filter(|(x, y)| 0 <= *x && *x < grid.width && 0 <= *y && *y < grid.height)
        .map(|(x, y)| ivec2(x, y))
        .collect_vec()
}

fn dijkstra<F: Fn(IVec2) -> i32>(grid: &mut Grid<i32>, seed: &[IVec2], cost: F) {
    let mut next: Vec<IVec2> = seed
        .iter()
        .flat_map(|pos| get_neighbors(*pos, grid))
        .collect_vec();

    while !next.is_empty() {
        let buffer = next.drain(..).collect_vec();
        for pos in buffer.into_iter() {
            let neighbor_max = {
                get_neighbors(pos, grid)
                    .into_iter()
                    .map(|pos| grid.get_clamped(pos.x, pos.y))
                    .max()
                    .cloned()
            };
            if let Some(neighbor_max) = neighbor_max {
                let v = *grid.get_clamped_v(pos);
                let c = cost(pos);
                if neighbor_max > v + c {
                    let new_val = neighbor_max - c;
                    *grid.get_mut(pos.x, pos.y) = new_val;
                    next.extend(
                        get_neighbors(pos, grid)
                            .into_iter()
                            .filter(|pos| *grid.get(pos.x, pos.y) < new_val - cost(*pos)),
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_neighbors_test() {
        let grid = Grid::new(10, 10, 0);
        let neighbors = get_neighbors(ivec2(1, 1), &grid);
        assert_eq!(4, neighbors.len());
        assert!(neighbors.contains(&ivec2(0, 1)));
        assert!(neighbors.contains(&ivec2(2, 1)));
        assert!(neighbors.contains(&ivec2(1, 0)));
        assert!(neighbors.contains(&ivec2(1, 2)));

        let neighbors = get_neighbors(ivec2(0, 0), &grid);
        assert_eq!(2, neighbors.len());
        assert!(neighbors.contains(&ivec2(0, 1)));
        assert!(neighbors.contains(&ivec2(1, 0)));
    }

    #[test]
    fn dijkstra_map_test() {
        // basic
        let mut grid = Grid::new(10, 10, 0);
        let pos = ivec2(5, 5);
        *grid.get_clamped_mut(pos.x, pos.y) = 5;
        dijkstra(&mut grid, &[pos], |_| 1);
        assert_eq!(2, *grid.get(2, 5));

        // higher cost
        let mut grid = Grid::new(10, 10, 0);
        let pos = ivec2(5, 5);
        *grid.get_clamped_mut(pos.x, pos.y) = 5;
        dijkstra(&mut grid, &[pos], |_| 2);
        assert_eq!(0, *grid.get(2, 5));
        assert_eq!(1, *grid.get(3, 5));

        // multiple seeds
        let mut grid = Grid::new(10, 10, 0);
        let pos = ivec2(5, 5);
        *grid.get_clamped_mut(pos.x, pos.y) = 5;
        let pos2 = ivec2(1, 4);
        *grid.get_clamped_mut(pos2.x, pos2.y) = 5;
        dijkstra(&mut grid, &[pos, pos2], |_| 1);
        assert_eq!(3, *grid.get(2, 5));
    }
}
