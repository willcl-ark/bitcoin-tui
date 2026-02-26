{
  description = "Bitcoin Core tui";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
  };

  outputs =
    {
      nixpkgs,
      rust-overlay,
      crane,
      ...
    }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAllSystems = nixpkgs.lib.genAttrs systems;

      mkPkgs =
        system:
        import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };

      mkCraneLib =
        system:
        let
          pkgs = mkPkgs system;
          rust = pkgs.rust-bin.stable.latest.default;
        in
        (crane.mkLib pkgs).overrideToolchain rust;
    in
    {
      nixosModules.default = import ./module.nix;

      packages = forAllSystems (
        system:
        let
          pkgs = mkPkgs system;
          craneLib = mkCraneLib system;
          src = craneLib.cleanCargoSource ./.;
          commonArgs = {
            inherit src;
            pname = "bitcoin-tui";
            version = "0.1.0";
          };
          cargoArtifacts = craneLib.buildDepsOnly commonArgs;
        in
        {
          default = craneLib.buildPackage (commonArgs // { inherit cargoArtifacts; });
        }
      );

      devShells = forAllSystems (
        system:
        let
          pkgs = mkPkgs system;
          rust = pkgs.rust-bin.stable.latest.default.override {
            extensions = [
              "rust-src"
              "rust-analyzer"
            ];
          };
        in
        {
          default = pkgs.mkShell {
            buildInputs = [
              rust
              pkgs.just
            ];
          };
        }
      );
    };
}
