{
  inputs = {
    nixpkgs.url  = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay = {
      url    = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, rust-overlay }:
    let
      systems       = [ "aarch64-darwin" "x86_64-darwin" "x86_64-linux" ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems f;
    in {
      devShells = forAllSystems (system:
        let
          pkgs   = import nixpkgs {
            inherit system;
            overlays = [ rust-overlay.overlays.default ];
          };
          linaro = import ./nix/linaro.nix { inherit pkgs system; };
          rust   = pkgs.rust-bin.stable.latest.default.override {
            targets = [ "arm-unknown-linux-gnueabihf" ];
          };
          mupdf' = pkgs.mupdf.overrideAttrs (old: {
            version = "1.27.0";
            src = pkgs.fetchurl {
              url  = "https://casper.mupdf.com/downloads/archive/mupdf-1.27.0-source.tar.gz";
              hash = "sha256-riRCQW3kmRgtN6UmxvorrMejvtWoiNETygSERITf58Y=";
            };
            postInstall = builtins.replaceStrings ["Version: 1.27.2"] ["Version: 1.27.0"] old.postInstall;
          });
        in {
          default = pkgs.mkShell {
            nativeBuildInputs = [ pkgs.pkg-config ];
            packages = [
              linaro
              rust
            ];
            buildInputs = [
              pkgs.SDL2
              pkgs.freetype
              pkgs.harfbuzz
              pkgs.djvulibre
              mupdf'
              mupdf'.dev
            ];
          };
        }
      );
    };
}
