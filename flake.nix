# use with `nix develop`
# this flake assumes x86_64-linux with wayland
{
  inputs = { nixpkgs.url = "github:NixOS/nixpkgs"; };
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
          pkg-config # locate C dependencies
          alsaLib # sound

          # these two can be used to speed up the build
          # add the following to .cargo/config.toml
          clang
          mold

          # extra tooling
          ldtk # level editor
          jq # extract stuff from json
          tracy # profiler, call with ~Tracy~
          cargo-flamegraph # more profiling :)
          # cargo-llvm-lines
        ];
        # stuff we need to run
        LD_LIBRARY_PATH = with pkgs;
          lib.makeLibraryPath [
            libxkbcommon # keyboard
            wayland
            libGL # OpenGL I think
            alsaLib # sound
          ];

        # making the linking go fast (thank you mr mold)
        shellHook = ''
          if [ ! -d ".cargo" ]; then
            echo "creating .cargo/config.toml"
            mkdir .cargo
cat << 'EOF' > .cargo/config.toml
[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-Clink-arg=-fuse-ld=mold"]
EOF
          fi
        '';
      };
    };
}
