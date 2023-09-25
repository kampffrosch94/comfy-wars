# use with `nix develop`
{
  inputs.flake-utils.url = "github:numtide/flake-utils";
  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let pkgs = nixpkgs.legacyPackages.${system};
      in {
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [ cargo rustc clang pkg-config alsaLib ldtk ];
          LD_LIBRARY_PATH = with pkgs;
            lib.makeLibraryPath [ libxkbcommon wayland libGL alsaLib ];
        };
      });
}
