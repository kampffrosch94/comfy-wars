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
    {(.id): .tileRect | [.x, .y] }
]
'

entities_def_exp := '
[
    .defs.entities[] |
    {(.identifier): ({
          tile_pos: .tileRect | [.x, .y],
        }
        + ( .fieldDefs | map({ (.identifier): .defaultOverride.params[0] }) | add )
    )}
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

ldtk_file := 'assets/comfy_wars.ldtk'

# grab sprites from ldtk
parse_sprites:
    @jq '{{sprites_exp}}' < {{ldtk_file}}

# grab entities definition from ldtk
parse_entities_def:
    @jq '{{entities_def_exp}}' < {{ldtk_file}}

# grab entities placed on the map from ldtk
parse_entities_map:
    @jq '{{entities_map_exp}}' < {{ldtk_file}}


# parse data from the .ldtk file and write it into jsons
write_parsed:
  @just parse_sprites > assets/sprites.json
  @just parse_entities_def > assets/entities_def.json
  @just parse_entities_map > assets/entities_map.json