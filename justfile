default:
    @just --list

# cargo run with tracy enabled
tracy:
    cargo run -F comfy/tracy

sprites_exp := '
[
    .defs.enums[] |
    select(.identifier == "sprite") |
    .values[] |
    {(.id): .tileRect }
]
'

entities_def_exp := '
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

entities_map_exp := '
[
    .levels[].layerInstances[].entityInstances[] |
    {(.__identifier): ({
            pos: .__grid ,
        }
        + ( .fieldInstances | map({ (.__identifier): .__value }) | add )
    )}
]
'

# grab sprites from ldtk
parse_sprites:
    @jq '{{sprites_exp}}' < assets/comfy_wars.ldtk

# grab entities definition from ldtk
parse_entities_def:
    @jq '{{entities_def_exp}}' < assets/comfy_wars.ldtk

# grab entities placed on the map from ldtk
parse_entities_map:
    @jq '{{entities_map_exp}}' < assets/comfy_wars.ldtk


# parse data from the .ldtk file and write it into jsons
write_parsed:
  @just parse_sprites > assets/sprites.json
  @just parse_entities_def > assets/entities_def.json
  @just parse_entities_map > assets/entities_map.json