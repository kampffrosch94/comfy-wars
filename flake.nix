{
  inputs = {
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nixpkgs.url = "nixpkgs/nixos-unstable";
  };
  outputs = { nixpkgs, fenix, ... }:
    let
      forAllSystems = function:
        # insert more systems here
        nixpkgs.lib.genAttrs [ "x86_64-linux" ] (system:
          function (import nixpkgs {
            inherit system;
            overlays = [ fenix.overlays.default ];
          }));

    in {
      devShells = forAllSystems (pkgs: {
        default = pkgs.mkShell {
          packages = with pkgs; [
            # rust stuff
            (with pkgs.fenix; with stable; combine [
              cargo
              clippy
              rust-src
              rustc
              rustfmt
              targets.wasm32-unknown-unknown.stable.rust-std
            ])

            clang
            mold
            trunk
            # rust-analyzer-nightly # optional

            # necessary to build
            pkg-config # locate C dependencies
            alsaLib # sound
            libxkbcommon # keyboard
            wayland

            # extra tooling
            ldtk # level editor
            jq # extract stuff from json
            tracy # profiler, call with ~Tracy~
            cargo-flamegraph # more profiling :)
            cargo-watch
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
      });
    };
}
