{
  description = "Moncer — money tracer";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    crane = {
      url = "github:ipetkov/crane";
    };

    flake-utils = {
      url = "github:numtide/flake-utils";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      crane,
      flake-utils,
      ...
    }:
    {
      inherit
        (flake-utils.lib.eachSystem [ "x86_64-linux" ] (
          system:
          let
            pkgs = import nixpkgs {
              inherit system;
              overlays = [
                rust-overlay.overlays.default
              ];
            };

            rustToolchain = pkgs.rust-bin.stable.latest.default.override {
              extensions = [
                "rust-analyzer"
                "rust-src"
                "clippy"
              ];
            };

            rustNightlyToolchain = pkgs.rust-bin.selectLatestNightlyWith (
              toolchain:
              toolchain.default.override {
                extensions = [
                  "rust-analyzer"
                  "rust-src"
                  "rust-std"
                  "clippy"
                ];
              }
            );

            buildInputsWith = (
              additionalPackages:
              [
              ]
              ++ additionalPackages
            );

            nativeBuildInputsWith = (
              additionalPackages:
              with pkgs;
              [
                pkg-config
              ]
              ++ additionalPackages
            );

            LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [
            ];

            mkDevShell = (
              rustToolchain:
              pkgs.mkShell {
                packages = with pkgs; [
                  sea-orm-cli
                ];

                buildInputsWith = buildInputsWith [ rustToolchain ];
                nativeBuildInputs = nativeBuildInputsWith [ rustToolchain ];

                shellHook = ''
                  export LD_LIBRARY_PATH="${LD_LIBRARY_PATH}"
                  zsh
                '';
              }
            );

            craneLib = toolchain: (crane.mkLib pkgs).overrideToolchain toolchain;

            unfilteredRoot = ./.;

            appSrc =
              toolchain:
              pkgs.lib.fileset.toSource {
                root = unfilteredRoot;
                fileset = pkgs.lib.fileset.unions [
                  ((craneLib toolchain).fileset.commonCargoSources unfilteredRoot)
                  ./crates/filetype/src/layout.pest
                ];
              };

            appCommonArgs = toolchain: {
              src = appSrc toolchain;
              strictDeps = true;

              buildInputs = buildInputsWith [
                toolchain

              ];
              nativeBuildInputs = nativeBuildInputsWith [
                toolchain
                pkgs.makeWrapper
              ];

              LD_LIBRARY_PATH = LD_LIBRARY_PATH;
            };
            appCargoArtifacts = toolchain: (craneLib toolchain).buildDepsOnly (appCommonArgs toolchain);

            buildMoncerApplication = (
              toolchain:
              (craneLib toolchain).buildPackage (
                (appCommonArgs toolchain)
                // {
                  cargoArtifacts = appCargoArtifacts toolchain;

                  postInstall = ''
                    wrapProgram $out/bin/moncer --prefix LD_LIBRARY_PATH : "${LD_LIBRARY_PATH}"
                  '';
                }
              )
            );
          in
          {
            devShells = {
              default = mkDevShell rustToolchain;
              nightly = mkDevShell rustNightlyToolchain;
            };

            packages = {
              default = buildMoncerApplication rustToolchain;
              nightly = buildMoncerApplication rustNightlyToolchain;
            };

            checks = {
              build = self.packages."${system}".default;

              moncer-test = (craneLib rustToolchain).cargoNextest (
                (appCommonArgs rustToolchain)
                // {
                  cargoArtifacts = appCargoArtifacts rustToolchain;
                  cargoNextestExtraArgs = "--workspace";
                }
              );

              moncer-clippy = (craneLib rustToolchain).cargoClippy (
                (appCommonArgs rustToolchain)
                // {
                  cargoArtifacts = appCargoArtifacts rustToolchain;
                  cargoClippyExtraArgs = "--all-targets -- --deny warnings";
                }
              );

              moncer-fmt = (craneLib rustToolchain).cargoFmt {
                src = appSrc rustToolchain;
              };
            };
          }
        ))
        devShells
        packages
        checks
        ;

      homeModules.default =
        {
          config,
          lib,
          pkgs,
          ...
        }:
        let
          moncer-rs = self.packages."${pkgs.stdenv.system}".default;
        in
        {
          options.programs.moncer-rs = {
            enable = lib.mkEnableOption "Moncer — money tracer";

            service = lib.mkOption {
              type = lib.types.bool;
              default = false;
              description = "Enable moncer systemd service";
            };
          };

          config = lib.mkIf config.programs.moncer-rs.enable {
            home.packages = [
              moncer-rs
            ];
          };
        };
    };
}
