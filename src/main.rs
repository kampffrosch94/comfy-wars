use comfy::*;

simple_game!("comfy wars", setup, update);

struct Player;
struct Ground;
struct Infrastructure;

fn setup(c: &mut EngineContext) {
    let ldtk = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/comfy_wars.ldtk"
    ));
    let ldtk: LDTK = DeJson::deserialize_json(ldtk).unwrap();
    dbg!(&ldtk);

    c.load_texture_from_bytes(
        "tilemap",
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/tilemap/tilemap_packed.png"
        )),
    );

    // Load the player texture
    c.load_texture_from_bytes(
        "player",
        include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/tiles/guy.png")),
    );


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
            Transform::position(vec2(tile.px[0]/GRIDSIZE as f32, -tile.px[1]/GRIDSIZE as f32 )),
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
            Transform::position(vec2(tile.px[0]/GRIDSIZE as f32, -tile.px[1]/GRIDSIZE as f32 )),
            Infrastructure,
        ));
    }

    // Spawn the player entity and make sure z-index is above the grass
    c.commands().spawn((
        Transform::position(vec2(0.0, 0.0)),
        Player,
        AnimatedSpriteBuilder::new()
            .z_index(10)
            .add_animation(
                "idle",
                0.1,
                true,
                AnimationSource::Atlas {
                    name: "player".into(),
                    offset: ivec2(0, 0),
                    step: ivec2(16, 0),
                    size: isplat(16),
                    frames: 1,
                },
            )
            .build(),
    ));
}

fn update(c: &mut EngineContext) {
    clear_background(TEAL);

    let dt = c.delta;

    for (_, (_, animated_sprite, transform)) in c
        .world()
        .query::<(&Player, &mut AnimatedSprite, &mut Transform)>()
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

        if moved {
            animated_sprite.flip_x = move_dir.x < 0.0;
            transform.position += move_dir.normalize() * speed * dt;
            animated_sprite.play("walk");
        } else {
            animated_sprite.play("idle");
        }

        main_camera_mut().center = transform.position;
    }


    let text = format!("fps: {}", get_fps());
    draw_text(&text, vec2(0.0, 1.0), WHITE, TextAlign::Center);
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
    #[nserde(rename = "intGridCsv")]
    int_grid: Vec<i64>,
    #[nserde(rename = "autoLayerTiles")]
    auto_tiles: Vec<AutoTile>,
}

#[derive(DeJson, Debug)]
struct AutoTile {
    px: [f32; 2],
    src: [i32; 2],
}
