mod data;
#[macro_use]
mod debug;
mod dijkstra;
mod game;
mod loading;

use comfy::*;
use cosync::{Cosync, CosyncInput, CosyncQueueHandle};
use data::*;
use debug::*;
use dijkstra::*;
use game::*;
use grids::*;
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
const Z_UNIT: i32 = 20;
const Z_UNIT_HP: i32 = 21;
const Z_CURSOR: i32 = 100;

pub struct GameWrapper {
    cosync: Cosync<GameState>,
    game_state: GameState,
}

impl GameWrapper {
    pub fn new(c: &mut EngineState) -> Self {
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
    phase: GamePhase,
}

impl GameState {
    pub fn new(_c: &mut EngineState, co: CosyncQueueHandle<GameState>) -> Self {
        Self {
            ui: Default::default(),
            sprites: Default::default(),
            entity_defs: Default::default(),
            grids: Default::default(),
            co,
            entities: Default::default(),
            phase: Default::default(),
        }
    }
}

#[derive(Debug, Default, PartialEq)]
enum GamePhase {
    #[default]
    PlayerPhase,
    EnemyPhase,
}

#[derive(Debug, Default)]
struct UIState {
    right_click_menu_pos: Option<Vec2>,
    draw_dijkstra_map: bool,
    draw_ai_map: bool,
    selected_entity: Option<Index>,
    move_state: MoveState,
    chosen_enemy: Option<usize>,
}

#[derive(Default, Debug, PartialEq, Eq)]
enum MoveState {
    #[default]
    None,
    Moving, // Animation

    Confirm,
    ChooseAttack,
    Attacking, // Animation
}

/// all grids in here have the same dimensions
/// bigger x is right
/// bigger y is down (reverse of what comfy uses atm)
#[derive(Debug)]
struct Grids {
    ground: Grid<GroundType>,
    terrain: Grid<TerrainType>,
}

impl Default for Grids {
    fn default() -> Self {
        Self {
            ground: Grid::new(0, 0, Default::default()),
            terrain: Grid::new(0, 0, Default::default()),
        }
    }
}

const GRIDSIZE: i32 = 16;

fn setup(s: &mut GameWrapper, c: &mut EngineContext) {
    let s = &mut s.game_state;
    // load tiles
    let ldtk: LDTK = DeJson::deserialize_json(kf_include_str!("/assets/comfy_wars.ldtk")).unwrap();
    {
        let level = &ldtk.levels[0];
        let (w, h) = (level.pixel_width / GRIDSIZE, level.pixel_height / GRIDSIZE);
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
            hp: HP_MAX,
            has_moved: false,
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

    draw_game(s);

    if s.phase == GamePhase::PlayerPhase {
        handle_input(s);
    }
    handle_debug_input(s);
}

fn draw_game(s: &mut GameState) {
    // draw actors
    for (_index, actor) in s.entities.iter() {
        draw_sprite_ex(
            texture_id("tilemap"),
            actor.draw_pos,
            if actor.has_moved { GRAY } else { WHITE },
            Z_UNIT,
            DrawTextureParams {
                dest_size: Some(vec2(1.0, 1.0).as_world_size()),
                source_rect: Some(IRect {
                    offset: actor.sprite_coords,
                    size: ivec2(GRIDSIZE, GRIDSIZE),
                }),
                ..Default::default()
            },
        );

        if actor.hp < 10 {
            let sprite = match actor.hp {
                0 => "hp_0",
                1 => "hp_1",
                2 => "hp_2",
                3 => "hp_3",
                4 => "hp_4",
                5 => "hp_5",
                6 => "hp_6",
                7 => "hp_7",
                8 => "hp_8",
                9 => "hp_9",
                _ => "hp_question",
            };
            cw_draw_sprite(s, sprite, actor.draw_pos, Z_UNIT_HP)
        }
    }
}

/// relevant for the actual game
/// also does drawing in immediate mode
fn handle_input(s: &mut GameState) {
    if is_mouse_button_released(MouseButton::Right) {
        s.ui.right_click_menu_pos = Some(mouse_world());
    }
    if is_mouse_button_released(MouseButton::Left) && s.ui.move_state == MoveState::None {
        s.ui.right_click_menu_pos = None;
        let pos = grid_world_pos(mouse_world());
        s.ui.selected_entity = None;

        for (key, actor) in s.entities.iter() {
            // I am scared of floats
            if pos.abs_diff_eq(actor.draw_pos, 0.01)
                && actor.team == PLAYER_TEAM
                && actor.has_moved == false
            {
                s.ui.selected_entity = Some(key);
            }
        }
    }

    if is_key_pressed(KeyCode::End) {
        s.phase = GamePhase::EnemyPhase;
        s.co.queue(enemy_phase);
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
                                        Z_UNIT,
                                        WHITE,
                                    )
                                    .with_rect(sprite.x, sprite.y, GRIDSIZE, GRIDSIZE),
                                ));
                            }
                        }
                    });
            });
    }

    if let Some(e) = s.ui.selected_entity {
        if s.ui.move_state == MoveState::None {
            let pos = s.entities[e].draw_pos;
            let team = s.entities[e].team;
            draw_cursor(s, pos);

            let start_pos = grid_pos(pos);
            // handle move range
            let mut move_range = Grid::new(s.grids.ground.width, s.grids.ground.height, 0);
            move_range[start_pos] = 9;
            dijkstra(&mut move_range, &[start_pos], movement_cost(s, PLAYER_TEAM));

            // find goal
            let mut grid = Grid::new(s.grids.ground.width, s.grids.ground.height, 0);
            let goal = mouse_game_grid();
            *grid.get_clamped_mut(goal.x, goal.y) = 99; // TODO increase this when done developing
            dijkstra(&mut grid, &[goal], movement_cost(s, PLAYER_TEAM));
            move_range.clamp_values(0, 1);
            grid.mul_inplace(&move_range);

            // allow passing through allies, but don't stop on them
            let mut seeds = Vec::new();
            for (_, actor) in &s.entities {
                grid[actor.pos] = -99;
                seeds.push(actor.pos);
            }
            let highest_reachable_pos = grid
                .iter_coords()
                .max_by_key(|(_pos, val)| *val)
                .map(|(pos, _)| pos)
                .unwrap();
            seeds.push(highest_reachable_pos);
            dijkstra(&mut grid, &seeds, movement_cost(s, PLAYER_TEAM));
            grid.mul_inplace(&move_range);

            // disallow moving through enemies
            for (_, actor) in s.entities.iter().filter(|(_i, a)| a.team != team) {
                grid[actor.pos] = -99;
            }
            draw_dijkstra_map(&grid);

            // finally actually calculate and draw the path
            let path = dijkstra_path(&grid, start_pos);
            draw_move_path(s, &path);

            draw_move_range(s, &grid);
            if s.ui.draw_dijkstra_map {
                draw_dijkstra_map(&grid);
            }

            if is_mouse_button_pressed(MouseButton::Left) && path.len() > 0 {
                s.ui.move_state = MoveState::Moving;
                s.co.queue(move |mut s| async move {
                    for pos in path.iter() {
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
                    let last = *path.last().unwrap();
                    let target = game_to_world(last);
                    let s = &mut s.get();
                    s.entities[e].draw_pos = target;
                    s.entities[e].pos = last;
                    s.ui.move_state = MoveState::Confirm;
                });
            }
            if is_key_pressed(KeyCode::Space) {
                // stand on the spot and attack or wait
                s.ui.move_state = MoveState::Confirm;
            }
        }
        if s.ui.move_state == MoveState::Confirm {
            let pos = world_to_screen(s.entities[e].draw_pos);
            egui::Area::new("move confirmation")
                .fixed_pos(egui::pos2(pos.x, pos.y))
                .show(egui(), |ui| {
                    egui::Frame::none()
                        .fill(egui::Color32::BLACK)
                        .show(ui, |ui| {
                            // TODO escape or right click reset to start
                            if ui.button("Wait").clicked() {
                                let e = s.ui.selected_entity.take().unwrap();
                                s.entities[e].has_moved = true;
                                s.ui.move_state = MoveState::None;
                            }

                            // check if unit from other team is in range
                            let enemies = enemies_in_range(s, e);
                            if enemies.len() > 0 {
                                if ui.button("Attack").clicked() {
                                    s.ui.move_state = MoveState::ChooseAttack;
                                }
                            }
                        })
                });
        }
        if s.ui.move_state == MoveState::ChooseAttack {
            let enemies = enemies_in_range(s, e);
            let chosen = s.ui.chosen_enemy.unwrap_or(0);
            let enemy = enemies[chosen];
            draw_cursor(s, game_to_world(enemy.1));
            draw_text(
                &format!("enemies: {}", enemies.len()),
                vec2(0., 0.),
                WHITE,
                TextAlign::Center,
            );
            if is_key_pressed(KeyCode::Escape) {
                // TODO revert one step instead
                s.ui.move_state = MoveState::None;
                s.ui.selected_entity = None;
            }
            if is_key_pressed(KeyCode::A) {
                s.ui.chosen_enemy = Some((chosen + 1) % enemies.len());
                println!("Switch enemy to {:?}", s.ui.chosen_enemy);
            }
            if is_key_pressed(KeyCode::Space) {
                s.ui.move_state = MoveState::Attacking;
                s.ui.chosen_enemy = None;
                s.co.queue(move |mut s| async move {
                    animate_attack(&mut s, e, enemy).await;

                    let s = &mut s.get();
                    let e = s.ui.selected_entity.take().unwrap();
                    s.entities[e].has_moved = true;
                    s.ui.move_state = MoveState::None;
                });
            }
        }
    } else {
        draw_cursor(s, mouse_world())
    }
}

async fn enemy_phase(mut s: cosync::CosyncInput<GameState>) {
    // reset has_moved
    for (_index, actor) in s.get().entities.iter_mut() {
        actor.has_moved = false;
    }

    let ai_units = s
        .get()
        .entities
        .iter()
        .filter(|(_k, a)| a.team == ENEMY_TEAM)
        .map(|e| e.0)
        .collect_vec();
    for index in ai_units {
        let (cursor, move_range, path, _grid) = {
            let s = &mut s.get();
            let actor = &s.entities[index];
            let start_pos = actor.pos;

            // handle move range
            let mut move_range = Grid::new(s.grids.ground.width, s.grids.ground.height, 0);
            move_range[start_pos] = 9;
            dijkstra(&mut move_range, &[start_pos], movement_cost(s, ENEMY_TEAM));
            move_range.clamp_values(0, 1);

            // find goal position
            let mut grid = Grid::new(s.grids.ground.width, s.grids.ground.height, 0);
            let enemy_positions = s
                .entities
                .iter()
                .filter(|(_i, a)| a.team == PLAYER_TEAM)
                .map(|(_i, a)| a.pos)
                .collect_vec();
            for pos in enemy_positions.iter() {
                grid[*pos] = 30;
            }
            dijkstra(&mut grid, &enemy_positions, movement_cost(s, ENEMY_TEAM));
            grid.mul_inplace(&move_range);

            // allow passing through allies, but don't stop on them
            // stop on your current position if its already the best
            for (_, actor) in s.entities.iter().filter(|(i, _)| *i != index) {
                grid[actor.pos] = -99;
            }

            let mut highest_reachable_pos = grid
                .iter_coords()
                .max_by_key(|(_pos, val)| *val)
                .map(|(pos, _)| pos)
                .unwrap();
            // stop weird running around that depends on grid iter order
            if grid[highest_reachable_pos] == grid[start_pos] {
                highest_reachable_pos = start_pos;
            }

            let mut grid = Grid::new(s.grids.ground.width, s.grids.ground.height, 0);
            grid[highest_reachable_pos] = 30;
            dijkstra(
                &mut grid,
                &[highest_reachable_pos],
                movement_cost(s, ENEMY_TEAM),
            );
            grid.mul_inplace(&move_range);
            dbg!(highest_reachable_pos);

            let debug_grid = grid.clone();

            // compute path
            let path = dijkstra_path(&grid, start_pos);

            // results for async usage
            (actor.draw_pos, move_range, path, debug_grid)
        };
        for _ in 0..tweak!(20) {
            {
                let s = &mut s.get();
                draw_move_range(s, &move_range);
                draw_cursor(s, cursor);
                draw_move_path(s, &path);
                draw_dijkstra_map(&_grid);
            }
            cosync::sleep_ticks(1).await;
        }
        // move along path
        {
            for pos in path.iter() {
                let target = game_to_world(*pos);
                let mut lerpiness = 0.;
                while lerpiness < 1. {
                    lerpiness += delta() * 25.;
                    {
                        let s = &mut s.get();
                        let drawpos = &mut s.entities[index].draw_pos;
                        *drawpos = drawpos.lerp(target, lerpiness);
                    }
                    cosync::sleep_ticks(1).await;
                }
            }
            let target = game_to_world(*path.last().unwrap());
            let s = &mut s.get();
            s.entities[index].draw_pos = target;
            s.entities[index].pos = *path.last().unwrap();
        }

        // attack player if close
        {
            cosync::sleep_ticks(20).await;
            for _ in 0..20 {
                cosync::sleep_ticks(1).await;
                let s = &mut s.get();
                if let Some((_enemy, pos)) = enemies_in_range(s, index).first() {
                    draw_cursor(s, game_to_world(*pos));
                }
            }

            if let Some(enemy) = {
                let s = &mut s.get();
                enemies_in_range(s, index).first()
            } {
                animate_attack(&mut s, index, *enemy).await;
            }
        }

        // mark as moved
        s.get().entities[index].has_moved = true;
    }

    s.get().phase = GamePhase::PlayerPhase;

    // reset has_moved
    for (_index, actor) in s.get().entities.iter_mut() {
        actor.has_moved = false;
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
            let actor = &s.entities[e];
            ui.label(format!("{:?}", actor));
        } else {
            ui.label("None");
        }
        ui.separator();
        ui.label(format!("Move State: {:?}", s.ui.move_state));

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

    if is_key_pressed(KeyCode::M) {
        s.ui.draw_ai_map = !s.ui.draw_ai_map;
    }

    cw_draw_debug_window();
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

fn draw_move_path(s: &GameState, path: &Vec<IVec2>) {
    const DOWN: (i32, i32) = (0, 1);
    const UP: (i32, i32) = (0, -1);
    const RIGHT: (i32, i32) = (1, 0);
    const LEFT: (i32, i32) = (-1, 0);

    let mut iter = path.iter();
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
        let prev = path[path.len() - 2];
        let pos = path[path.len() - 1];
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
}

fn movement_cost<'a>(s: &'a GameState, team: Team) -> impl Fn(IVec2) -> i32 + 'a {
    let blocked: HashSet<IVec2> = s
        .entities
        .iter()
        .filter_map(|(_i, e)| if e.team != team { Some(e.pos) } else { None })
        .collect();

    let cost_function = move |pos| -> i32 {
        if blocked.contains(&pos) {
            return 9999;
        }
        let ground = *s.grids.ground.get_clamped_v(pos);
        let terrain = *s.grids.terrain.get_clamped_v(pos);
        use GroundType as G;
        use TerrainType as T;
        match (ground, terrain) {
            (G::Water, _) => 9999,
            (G::Ground, T::None) => 2,
            (G::Ground, T::Street) => 1,
            (G::Ground, T::Forest) => 3,
        }
    };
    cost_function
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

fn draw_dijkstra_map(grid: &Grid<i32>) {
    for (x, y, val) in grid.iter() {
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

async fn animate_attack(s: &mut CosyncInput<GameState>, e: Index, enemy: (Index, IVec2)) {
    // insert attack animation here
    let start = s.get().entities[e].draw_pos;
    let target = game_to_world(enemy.1);
    let mut lerpiness = 0.;
    let speed = 5.;
    // forward
    while lerpiness < 0.5 {
        lerpiness += delta() * speed;
        {
            let s = &mut s.get();
            let drawpos = &mut s.entities[e].draw_pos;
            *drawpos = start.lerp(target, lerpiness);
        }
        cosync::sleep_ticks(1).await;
    }
    // backward
    while lerpiness >= 0.0 {
        lerpiness -= delta() * speed;
        {
            let s = &mut s.get();
            let drawpos = &mut s.entities[e].draw_pos;
            *drawpos = start.lerp(target, lerpiness);
        }
        cosync::sleep_ticks(1).await;
    }

    s.get().entities[e].draw_pos = start;
    let mut dmg = 5;
    while dmg > 0 {
        s.get().entities[enemy.0].hp -= 1;
        dmg -= 1;
        cosync::sleep_ticks(5).await;
    }

    if s.get().entities[enemy.0].hp <= 0 {
        // TODO animate death
        s.get().entities.remove(enemy.0);
    }

    // TODO attack back if still alive
}
