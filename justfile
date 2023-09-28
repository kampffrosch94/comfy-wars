default:
    @just --list

# cargo run with tracy enabled
tracy:
    cargo run -F comfy/tracy

# grab sprites from ldtk
parse_sprites:
    jq '.defs.enums[] | select(.identifier == "sprite") | .values[] | { id,  x: .tileRect.x , y: .tileRect.y } ' < assets/comfy_wars.ldtk | jq -s 'INDEX(.id)'  > assets/sprites.json
    jq . < assets/sprites.json



entities_exp := '
[
    .defs.entities[] |
    { (.identifier) : { 
        tile_x: .tileRect.x, 
        tile_y: .tileRect.y,
        team: ( .fieldDefs[] | select(.identifier == "team") | .defaultOverride.params[0] ),
        type: ( .fieldDefs[] | select(.identifier == "unit_type") | .defaultOverride.params[0] ),
        } 
    }
]
'

parse_entities:
    jq '{{entities_exp}}' < assets/comfy_wars.ldtk
