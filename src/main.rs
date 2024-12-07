#![allow(unused)]
mod data;
#[macro_use]
mod debug;
mod camera;
mod comfy_compat;
mod dijkstra;
mod egui_macroquad;
mod game;
mod grids;
mod loading;
mod util;

use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::HashSet;

use anyhow::Result;
use camera::CameraWrapper;
use comfy_compat::*;
use cosync::{Cosync, CosyncInput, CosyncQueueHandle};
use data::*;
use debug::*;
use dijkstra::*;
use egui::epaint;
use game::*;
use grids::*;
use inline_tweak::tweak;
use itertools::Itertools;
use loading::*;
use macroquad::prelude::*;
use nanoserde::*;
use serde::{Deserialize, Serialize};
use slotmap::{new_key_type, SlotMap};
use util::Vec2f;

fn window_conf() -> Conf {
    Conf {
        window_title: "comfy wars".to_owned(),
        fullscreen: false,
        high_dpi: true,
        sample_count: 0,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let game_wrapper = &mut GameWrapper::new();
    setup(game_wrapper).await.unwrap();
    loop {
        clear_background(BLACK);

        egui_macroquad::ui(|egui_ctx| {
            update(game_wrapper);
        });

        egui_macroquad::draw();
        next_frame().await
    }
}

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
const Z_DIJKSTRA_DEBUG: i32 = 30;
const Z_CURSOR: i32 = 100;

pub struct GameWrapper {
    cosync: Cosync<GameState>,
    game_state: GameState,
}

impl GameWrapper {
    pub fn new() -> Self {
        let cosync = Cosync::new();
        let handle = cosync.create_queue_handle();
        Self {
            cosync,
            game_state: GameState::new(handle),
        }
    }
}

#[derive(Clone)]
struct Sprite {
    params: DrawTextureParams,
    texture: Texture2D,
}

struct SpriteWithPos {
    params: DrawTextureParams,
    texture: Texture2D,
    pos: Vec2,
}

#[derive(Serialize, Deserialize)]
pub struct GameState {
    #[serde(skip, default = "null_object_queue_handle")]
    co: CosyncQueueHandle<GameState>,
    #[serde(skip)]
    draw_buffer: RefCell<Vec<DrawCommand>>,
    ui: UIState,
    #[serde(skip)]
    sprites: HashMap<String, Sprite>,
    #[serde(skip)]
    ground_sprites: Vec<SpriteWithPos>,
    #[serde(skip)]
    terrain_sprites: Vec<SpriteWithPos>,
    grids: Grids,
    entities: SlotMap<ActorKey, Actor>,
    phase: GamePhase,
    #[serde(skip)]
    camera: CameraWrapper,
}

struct DrawCommand {
    z_level: i32,
    command: Box<dyn FnOnce() -> ()>,
}

impl GameState {
    fn draw_sprite(&self, name: &str, dp: impl Into<Vec2f>, z_level: i32, color: Color) {
        let dp = dp.into();
        let sprite = (&self.sprites[name]).clone();
        let command = move || {
            draw_texture_ex(&sprite.texture, dp.x, dp.y, color, sprite.params);
        };
        self.draw_buffer.borrow_mut().push(DrawCommand {
            z_level,
            command: Box::new(command),
        });
    }

    fn draw_texture(
        &self,
        texture: Texture2D,
        dp: impl Into<Vec2f>,
        z_level: i32,
        color: Color,
        params: DrawTextureParams,
    ) {
        let dp = dp.into();
        let command = move || {
            draw_texture_ex(&texture, dp.x, dp.y, color, params);
        };
        self.draw_buffer.borrow_mut().push(DrawCommand {
            z_level,
            command: Box::new(command),
        });
    }

    fn draw_rect(&self, dp: impl Into<Vec2f>, w: f32, h: f32, z_level: i32, color: Color) {
        let dp = dp.into();
        let command = move || {
            draw_rectangle(dp.x, dp.y, GRIDSIZE as f32, GRIDSIZE as f32, color);
        };
        self.draw_buffer.borrow_mut().push(DrawCommand {
            z_level,
            command: Box::new(command),
        });
    }

    fn draw_text(
        &self,
        text: impl Into<String>,
        dp: impl Into<Vec2f>,
        z_level: i32,
        params: TextParams<'static>,
    ) {
        let dp = dp.into();
        let text = text.into();
        let command = move || {
            draw_text_ex(&text, dp.x, dp.y, params);
        };
        self.draw_buffer.borrow_mut().push(DrawCommand {
            z_level,
            command: Box::new(command),
        });
    }

    fn flush_draw_buffer(&mut self){
	let buffer = &mut self.draw_buffer.borrow_mut();
	buffer.sort_by_key(|it| it.z_level);
	for draw in buffer.drain(..) {
	    (draw.command)();
	}
    }
}

new_key_type! {
    struct ActorKey;
}

fn null_object_queue_handle() -> CosyncQueueHandle<GameState> {
    let cosync = Cosync::new();
    cosync.create_queue_handle()
}

impl GameState {
    pub fn new(co: CosyncQueueHandle<GameState>) -> Self {
        Self {
            draw_buffer: Default::default(),
            ui: Default::default(),
            sprites: Default::default(),
            grids: Default::default(),
            co,
            entities: Default::default(),
            phase: Default::default(),
            ground_sprites: Default::default(),
            terrain_sprites: Default::default(),
            camera: Default::default(),
        }
    }
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
enum GamePhase {
    #[default]
    PlayerPhase,
    EnemyPhase,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct UIState {
    #[serde(skip)] // TODO proxy
    right_click_menu_pos: Option<Vec2>,
    cursor_pos: Option<Vec2f>,
    last_mouse_pos: Vec2f,
    draw_dijkstra_map: bool,
    draw_ai_map: bool,
    selected_entity: Option<ActorKey>,
    move_state: MoveState,
    chosen_enemy: Option<usize>,
}

#[derive(Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Serialize, Deserialize)]
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

async fn setup(s: &mut GameWrapper) -> Result<()> {
    let s = &mut s.game_state;
    // load tiles
    let ldtk: LDTK = DeJson::deserialize_json(kf_include_str!("/assets/comfy_wars.ldtk")).unwrap();
    {
        let level = &ldtk.levels[0];
        let (w, h) = (level.pixel_width / GRIDSIZE, level.pixel_height / GRIDSIZE);
        s.grids.ground = Grid::new(w, h, Default::default());
    }

    let texture = load_texture("assets/tilemap/tilemap_packed.png")
        .await
        .expect("Tilemap not found");
    texture.set_filter(FilterMode::Nearest);

    // load sprites
    let sprites_str = kf_include_str!("/assets/sprites.json");
    let sprite_datas: HashMap<String, SpriteData> = DeJson::deserialize_json(sprites_str).unwrap();
    s.sprites = HashMap::new();
    for (name, data) in sprite_datas.into_iter() {
        let source_rect = Rect {
            x: data.x as _,
            y: data.y as _,
            w: GRIDSIZE as _,
            h: GRIDSIZE as _,
        };
        let sprite = Sprite {
            params: DrawTextureParams {
                source: Some(source_rect),
                ..Default::default()
            },
            texture: texture.clone(),
        };
        s.sprites.insert(name, sprite);
    }


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

        s.ground_sprites = layer
            .auto_tiles
            .iter()
            .map(|tile| {
                let source_rect = Rect {
                    x: tile.src[0] as _,
                    y: tile.src[1] as _,
                    w: GRIDSIZE as _,
                    h: GRIDSIZE as _,
                };
                SpriteWithPos {
                    params: DrawTextureParams {
                        source: Some(source_rect),
                        ..Default::default()
                    },
                    texture: texture.clone(),
                    pos: vec2(tile.px[0], tile.px[1]),
                }
            })
            .collect_vec();
    }

    for layer in ldtk
        .levels
        .iter()
        .flat_map(|level| level.layers.iter())
        .filter(|layer| layer.id == "infrastructuregrid")
    {
        s.grids.terrain = grid_from_layer(layer, |i| match i {
            0 => TerrainType::None,
            1 | 2 | 3 | 4 => TerrainType::Street,
            5 => TerrainType::Forest,
            _ => panic!("unsupported terrain type {}", i),
        });
        s.terrain_sprites = layer
            .auto_tiles
            .iter()
            .map(|tile| {
                let source_rect = Rect {
                    x: tile.src[0] as _,
                    y: tile.src[1] as _,
                    w: GRIDSIZE as _,
                    h: GRIDSIZE as _,
                };
                SpriteWithPos {
                    params: DrawTextureParams {
                        source: Some(source_rect),
                        ..Default::default()
                    },
                    texture: texture.clone(),
                    pos: vec2(tile.px[0], tile.px[1]),
                }
            })
            .collect_vec();
    }

    // load entity definitions
    let entity_defs: HashMap<String, EntityDef> =
        DeJson::deserialize_json(kf_include_str!("/assets/entities_def.json")).unwrap();

    // load entities on map
    let em = kf_include_str!("/assets/entities_map.json");
    let map_entities: Vec<EntityOnMap> = DeJson::deserialize_json(em).unwrap();

    for (name, def) in &entity_defs {
        let source_rect = Rect {
            x: def.sprite.x as _,
            y: def.sprite.y as _,
            w: GRIDSIZE as _,
            h: GRIDSIZE as _,
        };
        let params = DrawTextureParams {
            source: Some(source_rect),
            ..Default::default()
        };
        s.sprites.insert(
            name.clone(),
            Sprite {
                params,
                texture: texture.clone(),
            },
        );
    }

    for me in map_entities {
        let name = &me.def;
        let def = &entity_defs[&me.def];
        s.entities.insert(Actor {
            pos: me.pos.into(),
            draw_pos: vec2((me.pos[0] * GRIDSIZE) as f32, (me.pos[1] * GRIDSIZE) as f32),
            sprite_coords: ivec2(def.sprite.x, def.sprite.y),
            sprite_name: name.clone(),
            team: def.team,
            unit_type: def.unit_type,
            hp: HP_MAX,
            has_moved: false,
        });
    }

    Ok(())
}

fn update(s: &mut GameWrapper) {
    let co = &mut s.cosync;
    let s = &mut s.game_state;
    co.run_until_stall(s);
    let mut visuals = egui::Visuals::dark();
    visuals.window_shadow = epaint::Shadow {
        color: epaint::Color32::BLACK,
        offset: egui::Vec2::new(0., 0.),
        blur: 0.,
        spread: 0.,
    };
    egui().set_visuals(visuals);

    if is_key_pressed(KeyCode::F5) {
        println!("Saving game.");
    }
    if is_key_pressed(KeyCode::F9) {
        println!("Loading game.");
    }

    s.camera.process();
    draw_tiles(s);
    if s.phase == GamePhase::PlayerPhase {
        handle_input(s);
    }
    handle_debug_input(s);
    draw_actors(s);

    // TODO remove this indirection
    if let Some(pos) = s.ui.cursor_pos.take() {
        draw_cursor(s, pos.into());
    }

    cw_debug!("Draw calls buffered: {}", s.draw_buffer.borrow().len());
    s.flush_draw_buffer();
}

fn draw_tiles(s: &mut GameState) {
    for sprite in s.ground_sprites.iter() {
        let pos: Vec2f = sprite.pos.into();
        s.draw_texture(
            sprite.texture.clone(),
            pos,
            Z_GROUND,
            WHITE,
            sprite.params.clone(),
        )
    }
    for sprite in s.terrain_sprites.iter() {
        let pos: Vec2f = sprite.pos.into();
        s.draw_texture(
            sprite.texture.clone(),
            pos,
            Z_TERRAIN,
            WHITE,
            sprite.params.clone(),
        )
    }
}

fn draw_actors(s: &mut GameState) {
    // draw actors
    for (_index, actor) in s.entities.iter() {
        let color = if actor.has_moved { GRAY } else { WHITE };
        s.draw_sprite(&actor.sprite_name, actor.draw_pos, Z_UNIT_HP, color);

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
            s.draw_sprite(sprite, actor.draw_pos, Z_UNIT_HP, WHITE);
        }
    }
}

/// relevant for the actual game
/// also does drawing in immediate mode
fn handle_input(s: &mut GameState) {
    if is_mouse_button_down(MouseButton::Middle) {
        s.camera.mouse_delta(s.ui.last_mouse_pos, mouse_position());
    }

    s.ui.last_mouse_pos = mouse_position().into();
    match mouse_wheel() {
        (_x, y) => {
            if y != 0. {
                if y > 0. {
                    s.camera.zoom(1);
                }
                if y < 0. {
                    s.camera.zoom(-1);
                }
            }
        }
    }

    if is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift) {
        if is_key_pressed(KeyCode::W) {
            s.camera.move_camera((0., -32.));
        }
        if is_key_pressed(KeyCode::S) {
            s.camera.move_camera((0., 32.));
        }
        if is_key_pressed(KeyCode::A) {
            s.camera.move_camera((-32., 0.));
        }
        if is_key_pressed(KeyCode::D) {
            s.camera.move_camera((32., 0.));
        }
    }

    if is_mouse_button_released(MouseButton::Right) {
        s.ui.right_click_menu_pos = Some(s.camera.mouse_world());
    }
    if is_mouse_button_released(MouseButton::Left) && s.ui.move_state == MoveState::None {
        s.ui.right_click_menu_pos = None;
        let pos = grid_world_pos(s.camera.mouse_world());
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
        s.ui.cursor_pos = Some(wpos.into());
        let pos = s.camera.world_to_screen(wpos);
        egui::Area::new(egui::Id::new("context_menu"))
            .fixed_pos(egui::pos2(pos.x, pos.y))
            .show(&egui(), |ui| {
                egui::Frame::none()
                    .fill(egui::Color32::BLACK)
                    .show(ui, |ui| {
                        for (name, sprite) in s.sprites.iter().sorted_by_key(|s| s.0) {
                            if ui.button(name).clicked() {
                                /*
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
                                */ // TODO new right click menu
                            }
                        }
                    });
            });
    }

    if let Some(e) = s.ui.selected_entity {
        if s.ui.move_state == MoveState::None {
            let pos = s.entities[e].draw_pos;
            let team = s.entities[e].team;

            let start_pos = grid_pos(pos);
            // handle move range
            let mut move_range = Grid::new(s.grids.ground.width, s.grids.ground.height, 0);
            move_range[start_pos] = 9;
            dijkstra(&mut move_range, &[start_pos], movement_cost(s, PLAYER_TEAM));

            // find goal
            let mut grid = Grid::new(s.grids.ground.width, s.grids.ground.height, 0);
            let goal = mouse_game_grid(s);
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

            // finally actually calculate and draw the path
            let path = dijkstra_path(&grid, start_pos);
            draw_move_range(s, &grid);
            draw_move_path(s, &path);

            //draw_dijkstra_map(&grid);
            if s.ui.draw_dijkstra_map {
                draw_dijkstra_map(s, &grid);
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
                                *drawpos = drawpos.lerp(target.into(), lerpiness);
                            }
                            cosync::sleep_ticks(1).await;
                        }
                    }
                    let last = *path.last().unwrap();
                    let target = game_to_world(last);
                    let s = &mut s.get();
                    s.entities[e].draw_pos = target.into();
                    s.entities[e].pos = last;
                    s.ui.move_state = MoveState::Confirm;
                });
            }
            if is_key_pressed(KeyCode::Space) {
                // stand on the spot and attack or wait
                s.ui.move_state = MoveState::Confirm;
            }
            s.ui.cursor_pos = Some(pos.into());
        }
        if s.ui.move_state == MoveState::Confirm {
            let pos = s.camera.world_to_screen(s.entities[e].draw_pos);
            egui::Area::new(egui::Id::new("move confirmation"))
                .fixed_pos(egui::pos2(pos.x, pos.y))
                .show(&egui(), |ui| {
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

            // TODO
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
            s.ui.cursor_pos = Some(game_to_world(enemy.1).into());
        }
    } else {
        s.ui.cursor_pos = Some(s.camera.mouse_world().into());
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
                s.ui.cursor_pos = Some(cursor.into());
                draw_move_path(s, &path);
                draw_dijkstra_map(s, &_grid);
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
                    s.ui.cursor_pos = Some(game_to_world(*pos).into());
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
    egui::Window::new("kf_debug_info").show(&egui(), |ui| {
        let pos = grid_world_pos(s.camera.mouse_world());
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
    s.draw_sprite("cursor", grid_world_pos(pos), Z_CURSOR, WHITE);
}

fn draw_move_range(s: &GameState, grid: &Grid<i32>) {
    for (x, y, v) in grid.iter() {
        if *v > 0 {
            let pos = ivec2(x, y);
            let pos = game_to_world(pos);
            s.draw_sprite("move_range", pos, Z_MOVE_HIGHLIGHT, WHITE);
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
                s.draw_sprite(sprite, game_to_world(prev), Z_MOVE_ARROW, WHITE);
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
        s.draw_sprite(sprite, game_to_world(pos), Z_MOVE_ARROW, WHITE);
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

/// rounds pos to align with grid
fn grid_world_pos(v: Vec2) -> Vec2 {
    let mut pos = grid_pos(v);
    pos.x *= GRIDSIZE;
    pos.y *= GRIDSIZE;
    pos.as_vec2()
}

fn grid_pos(v: Vec2) -> IVec2 {
    let pos = Vec2 {
        x: v.x.round(),
        y: v.y.round(),
    };
    let mut r = pos.as_ivec2();
    r.y /= GRIDSIZE;
    r.x /= GRIDSIZE;
    r
}

fn world_to_game(v: Vec2) -> IVec2 {
    let v = grid_world_pos(v);
    ivec2(v.x as i32 / GRIDSIZE, v.y as i32 / GRIDSIZE)
}

fn game_to_world(v: IVec2) -> Vec2 {
    vec2((v.x * GRIDSIZE) as f32, (v.y * GRIDSIZE) as f32).into()
}

fn mouse_game_grid(s: &GameState) -> IVec2 {
    world_to_game(s.camera.mouse_world())
}

fn draw_dijkstra_map(s: &GameState, grid: &Grid<i32>) {
    for (x, y, val) in grid.iter() {
        let color = Color {
            r: 0.1,
            g: 0.1,
            b: 0.1,
            a: 0.5,
        };
        let pos = game_to_world(ivec2(x, y));
        draw_rectangle(pos.x, pos.y, GRIDSIZE as f32, GRIDSIZE as f32, color);
        // TODO
        if *val > 0 {
            let params = TextParams {
                font_size: 28 * 2,
                font_scale: 1.0 / 4.,
                ..Default::default()
            };
            let pos = Vec2f::from(pos)
                + Vec2f {
                    x: 0.,
                    y: GRIDSIZE as f32,
                };
            s.draw_text(val.to_string(), pos, Z_DIJKSTRA_DEBUG, params);
        }
    }
}

async fn animate_attack(s: &mut CosyncInput<GameState>, e: ActorKey, enemy: (ActorKey, IVec2)) {
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
