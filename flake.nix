# use with `nix develop`
{
  inputs.flake-utils.url = "github:numtide/flake-utils";
  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let pkgs = nixpkgs.legacyPackages.${system};
        libPath = with pkgs; lib.makeLibraryPath [
          libxkbcommon
          xorg.libX11
          xorg.libXcursor
          xorg.libXi
          xorg.libXrandr
          wayland
          libGL
          alsaLib
        ];
      in {
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            cargo
            rustc
            clang
            pkg-config
            cargo-limit
            xorg.libX11
            xorg.libXext
            xorg.libXcursor
            xorg.libXrandr
            xorg.libXi
            pkgconfig
            udev
            alsaLib
            ldtk
          ];
          LD_LIBRARY_PATH = libPath;
          buildInputs = with pkgs; [ openssl ];
        };
      });
}
