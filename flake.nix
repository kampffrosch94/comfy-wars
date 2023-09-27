# use with `nix develop`
# this flake assumes x86_64-linux with wayland
{
  inputs = { nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable"; };
  outputs = { self, nixpkgs, flake-utils }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
    in {
      devShells.${system}.default = pkgs.mkShell {
        packages = with pkgs; [
          # necessary to build
          cargo
          rustc
          #clang
          pkg-config # locate C dependencies
          alsaLib # sound

          # extra tooling
          ldtk # level editor
          jq # extract stuff from json
          tracy # profiler, call with ~Tracy~
          cargo-flamegraph # more profiling :)
        ];
        # stuff we need to run
        LD_LIBRARY_PATH = with pkgs;
          lib.makeLibraryPath [
            libxkbcommon # keyboard
            wayland
            libGL # OpenGL I think
            alsaLib # sound
          ];
      };
    };
}
