{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/release-25.11";

    crane = {
      url = "github:ipetkov/crane";
    };

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.rust-analyzer-src.follows = "";
    };

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
      };
    };

    flake-utils.url = "github:numtide/flake-utils";

    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, crane, fenix, flake-utils, advisory-db, rust-overlay, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        inherit (pkgs) lib;

        # Standard toolchain for Linux
        rustToolchain = pkgs.rust-bin.stable.latest.default;
        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

        # Toolchain with wasm target
        rustToolchainWasm = pkgs.rust-bin.stable.latest.default.override {
          targets = [ "wasm32-unknown-unknown" ];
        };
        craneLibWasm = (crane.mkLib pkgs).overrideToolchain rustToolchainWasm;

        # Toolchain with windows target
        rustToolchainWindows = pkgs.rust-bin.stable.latest.default.override {
          targets = [ "x86_64-pc-windows-gnu" ];
        };

        # Use cross-pkgs for Windows
        pkgsWindows = pkgs.pkgsCross.mingwW64;
        craneLibWindows = (crane.mkLib pkgsWindows).overrideToolchain rustToolchainWindows;

        src = lib.cleanSourceWith {
          src = craneLib.path ./.;
          filter = path: type: (craneLib.filterCargoSources path type) || (builtins.match ".*/assets/.*$" path != null) || (builtins.match ".*/dist/.*$" path != null);
        };

        commonArgs = {
          inherit src;
          nativeBuildInputs = with pkgs; [
            cmake
            makeWrapper
            pkg-config
          ];
        };

        linuxArgs = commonArgs // {
          buildInputs = with pkgs; [
            openssl
            libGL
            fontconfig
            stdenv.cc.cc
            zlib
            wayland
            libxkbcommon
            glew
            egl-wayland
            xorg.libX11
            xorg.libXcursor
            xorg.libXi
            xorg.libXrandr
            xorg.libxcb
            alsa-lib
          ];
        };

        wasmArgs = commonArgs // {
          CARGO_BUILD_TARGET = "wasm32-unknown-unknown";
          doCheck = false;
        };

        windowsArgs = commonArgs // {
          CARGO_BUILD_TARGET = "x86_64-pc-windows-gnu";
          # MinGW linkers need some help finding pthreads sometimes when linked via rustc
          CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUSTFLAGS = "-C link-arg=-liphlpapi -C link-arg=-lpthread";
          doCheck = false;
          buildInputs = with pkgsWindows.windows; [
            pthreads
          ];
        };

        cargoArtifacts = craneLib.buildDepsOnly linuxArgs;
        cargoArtifactsWasm = craneLibWasm.buildDepsOnly wasmArgs;
        cargoArtifactsWindows = craneLibWindows.buildDepsOnly windowsArgs;

        my-crate = craneLib.buildPackage (linuxArgs // {
          inherit cargoArtifacts;
          nativeBuildInputs = linuxArgs.nativeBuildInputs ++ [ pkgs.patchelf ];
          postInstall = ''
            patchelf --set-rpath "${lib.makeLibraryPath linuxArgs.buildInputs}" $out/bin/everythingrl
          '';
        });

        my-crate-windows = craneLibWindows.buildPackage (windowsArgs // {
          cargoArtifacts = cargoArtifactsWindows;
        });

        my-crate-wasm = craneLibWasm.buildPackage (wasmArgs // {
          cargoArtifacts = cargoArtifactsWasm;
          nativeBuildInputs = wasmArgs.nativeBuildInputs ++ [ pkgs.wasm-bindgen-cli pkgs.binaryen ];
          postInstall = ''
            mkdir -p $out/dist
            cp -r dist/* $out/dist/

            PROJECT_NAME="everythingrl"

            wasm-bindgen $out/bin/$PROJECT_NAME.wasm --out-dir $out/dist --target web --no-typescript

            sed -i "s/import \* as __wbg_star0 from 'env';//" $out/dist/$PROJECT_NAME.js
            sed -i "s/let wasm;/let wasm; export const set_wasm = (w) => wasm = w;/" $out/dist/$PROJECT_NAME.js
            sed -i "s/imports\['env'\] = __wbg_star0;/return imports.wbg\;/" $out/dist/$PROJECT_NAME.js
            sed -i "s/const imports = __wbg_get_imports();/return __wbg_get_imports();/" $out/dist/$PROJECT_NAME.js

            wasm-opt -Os $out/dist/''${PROJECT_NAME}_bg.wasm -o $out/dist/''${PROJECT_NAME}_bg.wasm

            rm -rf $out/bin
          '';
        });
      in
      {
        checks = {
          inherit my-crate;
          my-crate-clippy = craneLib.cargoClippy (linuxArgs // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- --deny warnings";
          });
          my-crate-fmt = craneLib.cargoFmt {
            inherit src;
          };
          my-crate-audit = craneLib.cargoAudit {
            inherit src advisory-db;
          };
          my-crate-nextest = craneLib.cargoNextest (linuxArgs // {
            inherit cargoArtifacts;
          });
        };

        packages = {
          default = my-crate;
          wasm = my-crate-wasm;
          windows = my-crate-windows;
        };

        apps.default = flake-utils.lib.mkApp {
          drv = my-crate;
        };

        devShells.default = pkgs.mkShell {
          inputsFrom = [ my-crate ];
          LD_LIBRARY_PATH = "${lib.makeLibraryPath linuxArgs.buildInputs}";

          nativeBuildInputs = with pkgs; [
            rustToolchainWasm
            lld
            uv
            python3
            binaryen
            gamescope
            wasm-bindgen-cli
          ];

          shellHook = ''
            if [[ $- == *i* ]]; then
              exec zsh
            fi
          '';
        };
      });
}
