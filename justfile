default:
    @just --list

# cargo run with tracy enabled
tracy:
    cargo run -F comfy/tracy

# grab sprites from ldtk
parse_sprites:
    #jq -e '.defs.enums[1].values[] | { id: .id,  x: .tileRect.x , y: .tileRect.y,  }' < assets/comfy_wars.ldtk > assets/sprites.jsonl
    jq '.defs.enums[]| select(.identifier == "sprite") | .values[] | { id,  x: .tileRect.x , y: .tileRect.y }' < assets/comfy_wars.ldtk  
