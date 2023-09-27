default:
    @just --list

# cargo run with tracy enabled
tracy:
    cargo run -F comfy/tracy

# grab sprites from ldtk
parse_sprites:
    jq '.defs.enums[]| select(.identifier == "sprite") | .values[] | { id,  x: .tileRect.x , y: .tileRect.y } ' < assets/comfy_wars.ldtk  | jq -s . > assets/sprites.json
    jq . < assets/sprites.json
