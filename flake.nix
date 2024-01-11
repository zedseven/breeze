{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    rust-overlay,
    crane,
  }: let
    system = "x86_64-linux";

    pkgs = import nixpkgs {
      inherit system;
      overlays = [rust-overlay.overlays.default];
    };

    inherit (nixpkgs) lib;

    rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

    craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;
    src = self;

    cargoArtifacts = craneLib.buildDepsOnly {
      inherit src;
    };

    crate = craneLib.buildPackage {
      inherit cargoArtifacts src;
    };

    crate-clippy = craneLib.cargoClippy {
      inherit cargoArtifacts src;
      cargoClippyExtraArgs = "-- --deny warnings --allow unused";
    };

    crate-fmt-check = craneLib.cargoFmt {
      inherit src;
    };
  in {
    packages.${system}.default = crate;
    checks.${system} = {
      inherit crate crate-clippy crate-fmt-check;
    };
    devShells.${system}.default = pkgs.mkShell rec {
      # Below are runtime dependencies loaded via `dlopen` - they do not show up with `ldd`
      # https://github.com/rust-windowing/winit/issues/493
      # https://github.com/emilk/egui/discussions/1587
      # https://www.reddit.com/r/bevy/comments/1136v35/has_anybody_managed_to_make_linux_staticish/
      buildInputs = with pkgs; [
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
      LD_LIBRARY_PATH = "${lib.makeLibraryPath buildInputs}";

      packages = [
        rustToolchain
      ];

      env = {
        RUST_BACKTRACE = "full";
      };

      shellHook = ''
        # Required for use by RustRover, since it doesn't find the toolchain or stdlib by using the PATH
        # RustRover must then be configured to look inside this symlink for the toolchain
        ln --symbolic --force --no-dereference --verbose "${rustToolchain}" "./.direnv/rust-toolchain"
      '';
    };
  };
}
