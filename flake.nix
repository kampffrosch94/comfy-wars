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
          clang
          pkg-config
          alsaLib
          # extra tooling
          ldtk
        ];
        # stuff we need to run
        LD_LIBRARY_PATH = with pkgs;
          lib.makeLibraryPath [ libxkbcommon wayland libGL alsaLib ];
      };
    };
}
