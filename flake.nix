{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
  };

  outputs = {
    nixpkgs,
    flake-parts,
    rust-overlay,
    crane,
    ...
  } @ inputs:
    flake-parts.lib.mkFlake {inherit inputs;} {
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];

      perSystem = {system, ...}: let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [rust-overlay.overlays.default];
        };

        inherit (nixpkgs) lib;

        # Below are runtime dependencies loaded via `dlopen` - they do not show up with `ldd`
        # https://github.com/rust-windowing/winit/issues/493
        # https://github.com/emilk/egui/discussions/1587
        # https://www.reddit.com/r/bevy/comments/1136v35/has_anybody_managed_to_make_linux_staticish/
        # It doesn't seem like `crane` supports optional values in `craneLib.buildPackage`, so both backends are included
        # https://github.com/ipetkov/crane/issues/586
        rpathLibs = with pkgs; [
          libxkbcommon
          libGL

          # WINIT_UNIX_BACKEND=wayland
          wayland

          # WINIT_UNIX_BACKEND=x11
          xorg.libXcursor
          xorg.libXrandr
          xorg.libXi
          xorg.libX11
        ];

        extraFileTypesFilter = path: _: builtins.match ".*\\.(vert|frag)$" path != null;
        cleanCargoSourceCustom = path: type: (extraFileTypesFilter path type) || (craneLib.filterCargoSources path type);

        mainProgram = "breeze";

        rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

        src = lib.cleanSourceWith {
          src = craneLib.path ./.;
          filter = cleanCargoSourceCustom;
        };

        cargoArtifacts = craneLib.buildDepsOnly {inherit src;};

        crate = craneLib.buildPackage {
          inherit cargoArtifacts src;

          strictDeps = true;

          fixupPhase = ''
            patchelf --set-rpath "${lib.makeLibraryPath rpathLibs}:$(patchelf --print-rpath $out/bin/${mainProgram})" $out/bin/${mainProgram}
          '';

          meta = {
            inherit mainProgram;
            description = "A tool for running presentations without fluff";
            homepage = "https://github.com/zedseven/breeze";
            license = with lib.licenses; [
              asl20
              mit
            ];
            platforms = lib.platforms.unix;
            maintainers = with lib.maintainers; [zedseven];
          };
        };

        crate-clippy = craneLib.cargoClippy {
          inherit cargoArtifacts src;
          cargoClippyExtraArgs = "-- --deny warnings --allow unused";
        };

        crate-fmt-check = craneLib.cargoFmt {inherit src;};
      in {
        packages.default = crate;

        checks = {
          inherit crate crate-clippy crate-fmt-check;
        };

        devShells.default = pkgs.mkShell {
          LD_LIBRARY_PATH = "${lib.makeLibraryPath rpathLibs}";

          packages = [rustToolchain];

          env.RUST_BACKTRACE = "full";

          shellHook = ''
            # Required for use by RustRover, since it doesn't find the toolchain or stdlib by using the PATH
            # RustRover must then be configured to look inside this symlink for the toolchain
            ln --symbolic --force --no-dereference --verbose "${rustToolchain}" "./.direnv/rust-toolchain"
          '';
        };
      };
    };
}
