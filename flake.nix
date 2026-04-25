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
        in {
          default = pkgs.mkShell {
            packages = [ linaro rust ];
          };
        }
      );
    };
}
