{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/release-24.05";

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

        craneLib = (crane.mkLib pkgs).overrideToolchain pkgs.rust-bin.stable.latest.default;
        src = lib.cleanSourceWith {
          src = craneLib.path ./.;
          filter = path: type: (craneLib.filterCargoSources path type) || (builtins.match ".*/assets/.*$" path != null);
        };
        craneLibWasm = craneLib.overrideToolchain (pkgs.rust-bin.stable.latest.default.override {
          targets = [ "wasm32-unknown-unknown" ];
        });

        # Common arguments can be set here to avoid repeating them later
        commonArgs = {
          inherit src;

          nativeBuildInputs = with pkgs; [
            cmake
            makeWrapper
          ];

          buildInputs = with pkgs; [
            # Add additional build inputs here
            openssl
            libGL
            fontconfig
            pkg-config
            stdenv.cc.cc
          ] ++ lib.optionals pkgs.stdenv.isLinux [
            wayland
            libxkbcommon
            glew-egl
            egl-wayland
            xorg.libX11
            xorg.libXcursor
            xorg.libXi
            xorg.libXrandr
            xorg.libxcb
            alsaLib
          ] ++ lib.optionals pkgs.stdenv.isDarwin [
            # Additional darwin specific inputs can be set here
            pkgs.libiconv
          ];

          # Additional environment variables can be set directly
          # MY_CUSTOM_VAR = "some value";
        };

        wasmArgs = {
          CARGO_BUILD_TARGET = "wasm32-unknown-unknown";
        };

        craneLibLLvmTools = craneLib.overrideToolchain
          (fenix.packages.${system}.complete.withComponents [
            "cargo"
            "llvm-tools"
            "rustc"
          ]);

        # Build *just* the cargo dependencies, so we can reuse
        # all of that work (e.g. via cachix) when running in CI
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        # Build the actual crate itself, reusing the dependency
        # artifacts from above.
        my-crate = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
          postInstall = ''
            wrapProgram "$out/bin/seven-drl-2024" --set LD_LIBRARY_PATH "${lib.makeLibraryPath commonArgs.buildInputs}";
          '';
        });
        # Build the actual crate itself, reusing the dependency
        # artifacts from above.
        my-crate-wasm = craneLibWasm.buildPackage (wasmArgs // commonArgs // {
          inherit cargoArtifacts;
          cargoTestCommand = "true";
        });
      in
      {
        checks = {
          # Build the crate as part of `nix flake check` for convenience
          inherit my-crate;

          # Run clippy (and deny all warnings) on the crate source,
          # again, resuing the dependency artifacts from above.
          #
          # Note that this is done as a separate derivation so that
          # we can block the CI if there are issues here, but not
          # prevent downstream consumers from building our crate by itself.
          my-crate-clippy = craneLib.cargoClippy (commonArgs // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- --deny warnings";
          });

          my-crate-doc = craneLib.cargoDoc (commonArgs // {
            inherit cargoArtifacts;
          });

          # Check formatting
          my-crate-fmt = craneLib.cargoFmt {
            inherit src;
          };

          # Audit dependencies
          my-crate-audit = craneLib.cargoAudit {
            inherit src advisory-db;
          };

          # Run tests with cargo-nextest
          # Consider setting `doCheck = false` on `my-crate` if you do not want
          # the tests to run twice
          my-crate-nextest = craneLib.cargoNextest (commonArgs // {
            inherit cargoArtifacts;
            partitions = 1;
            partitionType = "count";
          });
        };

        packages = {
          default = my-crate;
          wasm = my-crate-wasm;
          my-crate-llvm-coverage = craneLibLLvmTools.cargoLlvmCov (commonArgs // {
            inherit cargoArtifacts;
          });
        };

        apps.default = flake-utils.lib.mkApp {
          drv = my-crate;
        };

        devShells.default = craneLibWasm.devShell
          {
            # Inherit inputs from checks.
            checks = self.checks.${system};

            # Additional dev-shell environment variables can be set directly
            # MY_CUSTOM_DEVELOPMENT_VAR = "something else";
            LD_LIBRARY_PATH = "${lib.makeLibraryPath commonArgs.buildInputs}";
            CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_LINKER = "lld";

            # Extra inputs can be added here; cargo and rustc are provided by default.
            packages = with pkgs; [
                lld
            ];
          };
      });
}

