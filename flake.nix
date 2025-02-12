{
  description = "The clock Rust library";
  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    embassy = {
      url = "github:embassy-rs/embassy";
      flake = false;
    };
  };

  outputs = inputs@{ self, nixpkgs, crane, flake-utils, rust-overlay, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        rustTarget = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        unstableRustTarget = pkgs.rust-bin.selectLatestNightlyWith (toolchain: toolchain.default.override {
          extensions = [ "rust-src" "miri" "rustfmt" ];
        });
        craneLib = (crane.mkLib pkgs).overrideToolchain rustTarget;
        unstableCraneLib = (crane.mkLib pkgs).overrideToolchain unstableRustTarget;

        tomlInfo = craneLib.crateNameFromCargoToml { cargoToml = ./Cargo.toml; };
        inherit (tomlInfo) pname version;
        src = ./.;

        rustfmt' = pkgs.writeShellScriptBin "rustfmt" ''
          exec "${unstableRustTarget}/bin/rustfmt" "$@"
        '';

        cargoArtifacts = craneLib.buildDepsOnly {
          inherit src;
          cargoExtraArgs = "--all-features --all";
        };

        clock = craneLib.buildPackage {
          inherit cargoArtifacts src version;
          cargoExtraArgs = "--all-features --all";
        };

      in
      rec {
        checks = {
          inherit clock;

          clock-clippy = craneLib.cargoClippy {
            inherit cargoArtifacts src;
            cargoExtraArgs = "--all --all-features";
            cargoClippyExtraArgs = "-- --deny warnings";
          };

          clock-fmt = unstableCraneLib.cargoFmt {
            inherit src;
          };
        };

        packages.clock = clock;
        packages.default = packages.clock;

        apps.default = flake-utils.lib.mkApp {
          name = "clock";
          drv = clock;
        };

        devShells.default = pkgs.mkShell {
          CYW43_FIRMWARE_BIN = "${inputs.embassy}/cyw43-firmware/43439A0.bin";
          CYW43_FIRMWARE_CLM_BIN = "${inputs.embassy}/cyw43-firmware/43439A0_clm.bin";

          nativeBuildInputs = [
            rustfmt'
            rustTarget

            pkgs.probe-rs
            pkgs.rerun
            pkgs.gitlint
          ];
        };
      }
    );
}
