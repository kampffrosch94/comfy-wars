use comfy::{egui::RichText, epaint::Color32, *};
use nanoserde::*;

simple_game!("comfy wars", GameState, setup, update);

// ECS markers
struct Player;
struct Ground;
struct Infrastructure;


pub struct GameState;

impl GameState {
    pub fn new(_c: &mut EngineContext) -> Self {
        Self {  }
    }
}


fn setup(s: &mut GameState, c: &mut EngineContext) {
    // can be turned on by hitting F8
    c.config.borrow_mut().dev.show_fps = false;

    // load tiles
    let ldtk = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/comfy_wars.ldtk"
    ));
    let ldtk: LDTK = DeJson::deserialize_json(ldtk).unwrap();

    c.load_texture_from_bytes(
        "tilemap",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/tilemap/tilemap_packed.png"
        )),
    );

    // load sprites
    let sprites_str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/sprites.json"));
    let sprites: HashMap<String, SpriteData> = DeJson::deserialize_json(sprites_str).unwrap();
    dbg!(&sprites);

    const GRIDSIZE: i32 = 16;
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

    // Spawn the player entity and make sure z-index is above the grass
    c.commands().spawn((
        Transform::position(vec2(0.0, 0.0)),
        Player,
        Sprite::new("tilemap".to_string(), vec2(1.0, 1.0), 10, WHITE).with_rect(
            sprites["blue_tank"].x,
            sprites["blue_tank"].y,
            GRIDSIZE,
            GRIDSIZE,
        ),
    ));
}

fn update(s: &mut GameState, c: &mut EngineContext) {
    span_with_timing!("kf/update");
    let _span = span!("renderer update");
    clear_background(TEAL);

    let dt = c.delta;

    for (_, (_, sprite, transform)) in c
        .world()
        .query::<(&Player, &mut Sprite, &mut Transform)>()
        .iter()
    {
        // Handle movement and animation
        let mut moved = false;
        let speed = 8.0;
        let mut move_dir = Vec2::ZERO;

        if is_key_down(KeyCode::W) {
            move_dir.y += 1.0;
            moved = true;
        }
        if is_key_down(KeyCode::S) {
            move_dir.y -= 1.0;
            moved = true;
        }
        if is_key_down(KeyCode::A) {
            move_dir.x -= 1.0;
            moved = true;
        }
        if is_key_down(KeyCode::D) {
            move_dir.x += 1.0;
            moved = true;
        }

        let v = move_dir.normalize_or_zero() * speed * dt;
        let vpers = get_fps() as f32 * v;
        let text = format!("speed per second: [{:.8},{:.8}]", vpers.x, vpers.y);
        draw_text(&text, vec2(0.0, 3.0), WHITE, TextAlign::Center);
        if moved {
            sprite.flip_x = move_dir.x < 0.0;
            transform.position += v;
            assert!(!transform.position.is_nan());
        }
        main_camera_mut().center = transform.position;
        //println!("Still trying to draw. {}", main_camera().center);
    }

    let mut visuals = egui::Visuals::dark();
    visuals.window_shadow = epaint::Shadow {
        extrusion: 0.,
        color: epaint::Color32::BLACK,
    };
    c.egui.set_visuals(visuals);
    let mouse = mouse_screen();
    egui::Area::new("my_area")
        .fixed_pos(egui::pos2(mouse.x, mouse.y))
        .show(c.egui, |ui| {
            egui::Frame::none().fill(egui::Color32::RED).show(ui, |ui| {
                ui.label(RichText::new("Red text").color(Color32::BLACK));
            });
        });

    let text = format!("fps: {}", get_fps());
    draw_text(&text, vec2(0.0, 1.0), WHITE, TextAlign::Center);
    let text = format!("dt: {:.8}", dt);
    draw_text(&text, vec2(0.0, 2.0), WHITE, TextAlign::Center);
}

#[derive(DeJson, Debug)]
struct SpriteData {
    x: i32,
    y: i32,
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
