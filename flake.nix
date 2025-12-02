{
  description = "The Rust clock server";
  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    inputs@{
      self,
      nixpkgs,
      crane,
      flake-utils,
      rust-overlay,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        rustTarget = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

        unstableRustTarget = pkgs.rust-bin.selectLatestNightlyWith (
          toolchain:
          toolchain.default.override {
            extensions = [
              "rust-src"
              "miri"
              "rustfmt"
            ];
          }
        );

        craneLib = (crane.mkLib pkgs).overrideToolchain rustTarget;

        unstableCraneLib = (crane.mkLib pkgs).overrideToolchain unstableRustTarget;

        tomlInfo = craneLib.crateNameFromCargoToml { cargoToml = ./Cargo.toml; };

        src =
          let
            markdownFilter = path: _type: pkgs.lib.hasSuffix ".md" path;
            filterPath =
              path: type:
              builtins.any (f: f path type) [
                markdownFilter
                craneLib.filterCargoSources
                pkgs.lib.cleanSourceFilter
              ];
          in
          pkgs.lib.cleanSourceWith {
            src = ./.;
            filter = filterPath;
          };

        rustSrc = pkgs.lib.fileset.toSource {
          root = ./.;
          fileset =
            let
              includeFilesWithExt = ext: (pkgs.lib.fileset.fileFilter (file: file.hasExt ext) ./.);
            in
            pkgs.lib.fileset.unions (
              [
                ./Cargo.lock
                ./README.md
              ]
              ++ (builtins.map includeFilesWithExt [
                "rs"
                "toml"
                "sql"
              ])
            );
        };

        rustfmt' = pkgs.writeShellScriptBin "rustfmt" ''
          exec "${unstableRustTarget}/bin/rustfmt" "$@"
        '';

        buildInputs = [ pkgs.openssl ];
        nativeBuildInputs = [
          pkgs.cmake
          pkgs.pkg-config
        ];

        cargoArtifacts = craneLib.buildDepsOnly {
          src = rustSrc;
          cargoExtraArgs = "--all-features --all";
          inherit buildInputs;
          inherit nativeBuildInputs;
        };

        clock = craneLib.buildPackage {
          inherit cargoArtifacts;
          src = rustSrc;
          version = tomlInfo.version;
          cargoExtraArgs = "--all-features --all";
          inherit buildInputs;
          inherit nativeBuildInputs;
        };

      in
      rec {
        checks = {
          inherit clock;

          clock-clippy = craneLib.cargoClippy {
            inherit cargoArtifacts;
            src = rustSrc;
            cargoExtraArgs = "--all --all-features";
            cargoClippyExtraArgs = "-- --deny warnings";
            inherit buildInputs;
            inherit nativeBuildInputs;
          };

          clock-fmt = unstableCraneLib.cargoFmt {
            src = rustSrc;
          };
        };

        packages = {
          inherit clock;
          default = packages.clock;
        };

        apps.default = flake-utils.lib.mkApp {
          name = "clock";
          drv = clock;
        };

        devShells.default = pkgs.mkShell {
          inherit buildInputs;
          nativeBuildInputs = nativeBuildInputs ++ [
            rustfmt'
            rustTarget
            pkgs.cargo-insta

            pkgs.gitlint
          ];
        };
      }
    );
}
