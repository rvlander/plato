{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

  outputs = { self, nixpkgs }:
    let
      systems       = [ "aarch64-darwin" "x86_64-darwin" "x86_64-linux" ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems f;
    in {
      devShells = forAllSystems (system:
        let
          pkgs   = import nixpkgs { inherit system; };
          linaro = import ./nix/linaro.nix { inherit pkgs system; };
        in {
          default = pkgs.mkShell {
            packages = [ linaro ];
          };
        }
      );
    };
}
