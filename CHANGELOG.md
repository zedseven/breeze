# Changelog

All notable changes to this project will be documented in this file.

## [1.2.1] - 2025-11-05

### Bug Fixes

- Fix an issue with touchscreen support where the middle of the window was not calculated correctly. ([929a59b4](https://github.com/zedseven/breeze/commit/929a59b4))
- Render transparent images without a background. ([7639f1ef](https://github.com/zedseven/breeze/commit/7639f1ef))

### Continuous Integration

- Change the generated artifacts' names to be more descriptive and to be closer to the release artifact names. ([1fa9657d](https://github.com/zedseven/breeze/commit/1fa9657d))

### Miscellaneous Tasks

- Update the copyright year. ([706da6b3](https://github.com/zedseven/breeze/commit/706da6b3))

## [1.2.0] - 2025-11-05

### Bug Fixes

- Handle errors that may occur when attempting to load the icon from the program's resources on startup, on Windows. ([32e8b066](https://github.com/zedseven/breeze/commit/32e8b066))

### Continuous Integration

- Update the `upload-artifact` action to version 4. ([088759fd](https://github.com/zedseven/breeze/commit/088759fd))

### Features

- Display errors when failing to parse the foreground or background colours specified in the presentation file. ([cf7590c3](https://github.com/zedseven/breeze/commit/cf7590c3))
- Allow the cursor to be shown over the presentation with a configuration setting inside the presentation file. ([21518693](https://github.com/zedseven/breeze/commit/21518693))
- Set the window icon on Windows. ([610b6c74](https://github.com/zedseven/breeze/commit/610b6c74))
- Add support for inverting the foreground and background colours of a presentation. ([e791d7a2](https://github.com/zedseven/breeze/commit/e791d7a2))
- Change the usable space used for scaling to 2/3 of the width and height, instead of 3/4. ([143f30d2](https://github.com/zedseven/breeze/commit/143f30d2))
- Add support for touching the screen to advance the presentation. ([3518b48f](https://github.com/zedseven/breeze/commit/3518b48f))
- Change the usable space used for scaling to 1/φ of the width and height, instead of 2/3. ([c4fbe1c5](https://github.com/zedseven/breeze/commit/c4fbe1c5))

### Miscellaneous Tasks

- Package the program for use on NixOS. ([09e4b59a](https://github.com/zedseven/breeze/commit/09e4b59a))
- Fix the regular expression used to filter files in the Nix build process. ([fd73411d](https://github.com/zedseven/breeze/commit/fd73411d))
- Add support for multiple architectures to the Nix flake. ([675385dd](https://github.com/zedseven/breeze/commit/675385dd))
- Update the Rust toolchain version to `nightly-2025-10-01`. ([da202873](https://github.com/zedseven/breeze/commit/da202873))
- Update `winit`-related dependencies. ([3af2475a](https://github.com/zedseven/breeze/commit/3af2475a))
- Update `winresource`-related dependencies. ([09dbbe70](https://github.com/zedseven/breeze/commit/09dbbe70))
- Fix an unformatted line of code. ([aa76570b](https://github.com/zedseven/breeze/commit/aa76570b))

### Refactor

- Apply Clippy lints. ([a0d37019](https://github.com/zedseven/breeze/commit/a0d37019))
- Apply Clippy lints that were suggested after updating the Rust toolchain version. ([473b6470](https://github.com/zedseven/breeze/commit/473b6470))

### Testing

- Update the test cases with the new changes. ([42f714a6](https://github.com/zedseven/breeze/commit/42f714a6))

## [1.1.1] - 2024-04-13

### Continuous Integration

- Only compile artifacts when new tags are pushed. ([806b5cfa](https://github.com/zedseven/breeze/commit/806b5cfa))

### Miscellaneous Tasks

- Remove the use of `nightly` Rust features so that the program can be compiled with stable Rust. ([6549eed1](https://github.com/zedseven/breeze/commit/6549eed1))

## [1.1.0] - 2024-04-13

### Bug Fixes

- Prevent a crash when the presentation file is empty. ([38e32c32](https://github.com/zedseven/breeze/commit/38e32c32))

### Continuous Integration

- Update a few actions to newer versions. ([a89c0454](https://github.com/zedseven/breeze/commit/a89c0454))
- Add a new check for running unit tests. ([7d7eaa86](https://github.com/zedseven/breeze/commit/7d7eaa86))

### Features

- Display user errors using the presentation interface. This makes the program much more user-friendly, and facilitates non-CLI use. ([d17ddcf4](https://github.com/zedseven/breeze/commit/d17ddcf4))
- Adjust the wording in the example presentation slightly. ([a0c2ac16](https://github.com/zedseven/breeze/commit/a0c2ac16))
- Add functionality to toggle fullscreen, using the `F11` key. ([6e28cd0d](https://github.com/zedseven/breeze/commit/6e28cd0d))

### Miscellaneous Tasks

- Update `rust-fontconfig` to an official release, now that the changes have been merged upstream. ([9b931a8f](https://github.com/zedseven/breeze/commit/9b931a8f))
- Fix an unformatted line of code. ([ed34ce90](https://github.com/zedseven/breeze/commit/ed34ce90))

### Testing

- Add unit tests for the presentation-parsing components. ([a7e0a42d](https://github.com/zedseven/breeze/commit/a7e0a42d))

## [1.0.0] - 2024-01-17

### Bug Fixes

- Prevent a potential panic when creating the window title. ([1da941f7](https://github.com/zedseven/breeze/commit/1da941f7))
- Perform the scaling calculations with a much smaller starting size to avoid wrapping that throws them off. ([c9504756](https://github.com/zedseven/breeze/commit/c9504756))
- Center tall columns of text in the middle of the screen, instead of aligning them to the left. ([8ed50c44](https://github.com/zedseven/breeze/commit/8ed50c44))
- Use nearest-neighbour filtering when the image had to be scaled up over 4x. Otherwise, use anisotropic 16x filtering to sample the image. ([b171192e](https://github.com/zedseven/breeze/commit/b171192e))
- Accommodate floating-point imprecision in the horizontal text bounds, preventing unexpected text wrapping. ([7407bea6](https://github.com/zedseven/breeze/commit/7407bea6))
- Fix a bug with image rendering where window resizes caused the scaling to be thrown off. ([968b9be3](https://github.com/zedseven/breeze/commit/968b9be3))
- Re-order the rect vertices in the rendering process. ([42a71e8c](https://github.com/zedseven/breeze/commit/42a71e8c))
- Convert sRGB colour values to linear RGB before using them for rendering. ([1bc5e382](https://github.com/zedseven/breeze/commit/1bc5e382))
- Remove `PragmataPro Mono Liga` from the example presentation. ([493241c6](https://github.com/zedseven/breeze/commit/493241c6))
- Fix a mistake in `build.rs`. ([77a27302](https://github.com/zedseven/breeze/commit/77a27302))
- Set `windows_subsystem` so that the program doesn't open a terminal window when run on Windows. ([fe9eda87](https://github.com/zedseven/breeze/commit/fe9eda87))

### Documentation

- Update the tagline. ([8ccdcc1f](https://github.com/zedseven/breeze/commit/8ccdcc1f))
- Add an example presentation. ([da14cdbe](https://github.com/zedseven/breeze/commit/da14cdbe))

### Features

- Initial commit. ([76f4de40](https://github.com/zedseven/breeze/commit/76f4de40))
- Read the file path from the command line. ([dfa1429f](https://github.com/zedseven/breeze/commit/dfa1429f))
- Implement the base for opening a window and displaying content. ([494d6ab9](https://github.com/zedseven/breeze/commit/494d6ab9))
- Implement text scaling to fit the usable space. ([a4fa3f5d](https://github.com/zedseven/breeze/commit/a4fa3f5d))
- Display text slides and include controls to switch between them. ([979aae3a](https://github.com/zedseven/breeze/commit/979aae3a))
- Set default colours. ([e3004e45](https://github.com/zedseven/breeze/commit/e3004e45))
- Set up the other keybindings that `sent` supports for navigating through the presentation. ([0789604a](https://github.com/zedseven/breeze/commit/0789604a))
- Use the contents of the first slide as the window title. ([84971942](https://github.com/zedseven/breeze/commit/84971942))
- Load and cache the slide images in memory on startup. ([d22564f0](https://github.com/zedseven/breeze/commit/d22564f0))
- Implement the base for rendering textures. More work is still required. ([eee08841](https://github.com/zedseven/breeze/commit/eee08841))
- Scale and display images properly. ([c5b2f85a](https://github.com/zedseven/breeze/commit/c5b2f85a))
- Load fonts from the system by name. ([804bd030](https://github.com/zedseven/breeze/commit/804bd030))
- Load fonts from the system using a list of names, and taking the first one that is found. ([60b1ab2c](https://github.com/zedseven/breeze/commit/60b1ab2c))
- Allow the user to set the fonts and foreground & background colours in the presentation file. ([f9e74ec0](https://github.com/zedseven/breeze/commit/f9e74ec0))
- Extend the default font list to theoretically cover all supported platforms. ([b3d31e07](https://github.com/zedseven/breeze/commit/b3d31e07))
- Open the presentation in borderless fullscreen. ([be1f25d3](https://github.com/zedseven/breeze/commit/be1f25d3))
- Redraw the window when it regains focus. ([9648b21b](https://github.com/zedseven/breeze/commit/9648b21b))
- Hide the mouse cursor in the presentation window. ([4e1df650](https://github.com/zedseven/breeze/commit/4e1df650))

### Miscellaneous Tasks

- Update the copyright year in `LICENSE-MIT`. ([d3c646ce](https://github.com/zedseven/breeze/commit/d3c646ce))
- Add `cargo` as a required component in `rust-toolchain.toml`. ([b0e609c2](https://github.com/zedseven/breeze/commit/b0e609c2))
- Add the runtime dependencies required for display to `flake.nix`. ([c4c05326](https://github.com/zedseven/breeze/commit/c4c05326))
- Update dependencies. ([d66bc682](https://github.com/zedseven/breeze/commit/d66bc682))
- Add a logo. ([7bf139a5](https://github.com/zedseven/breeze/commit/7bf139a5))
- Set up executable packaging for Windows so that the executable has its icon set to the new logo. ([815a4326](https://github.com/zedseven/breeze/commit/815a4326))
- Add additional Windows resource properties to `Cargo.toml`. ([a0f6aab6](https://github.com/zedseven/breeze/commit/a0f6aab6))
- Set the Windows resource property `FileDescription` to the application name, since Windows uses it as such in the Task Manager. ([41a9b368](https://github.com/zedseven/breeze/commit/41a9b368))
- Update `cliff.toml` to use a regular expression for `tag_pattern`. ([e4869beb](https://github.com/zedseven/breeze/commit/e4869beb))
- Update `cliff.toml` to only use the first line of each commit message. ([88365dcb](https://github.com/zedseven/breeze/commit/88365dcb))

### Performance

- Only render when a redraw is requested. ([a1c701bd](https://github.com/zedseven/breeze/commit/a1c701bd))

### Refactor

- Apply several Clippy lints. ([4a545abb](https://github.com/zedseven/breeze/commit/4a545abb))
- Move `PipelineOption` into its own module. ([61150092](https://github.com/zedseven/breeze/commit/61150092))
- Move scaling factor calculations into a new function, `calculate_scaling_factor`. ([d49f3471](https://github.com/zedseven/breeze/commit/d49f3471))
- Move image loading into its own function, `load_images_from_presentation`. ([761e4f03](https://github.com/zedseven/breeze/commit/761e4f03))
- Move all rendering functionality into its own module, `renderer`. ([2d81c730](https://github.com/zedseven/breeze/commit/2d81c730))
- Clean up `renderer`. ([0fe4f882](https://github.com/zedseven/breeze/commit/0fe4f882))
- Move `main` up in `main.rs`. ([49873c3f](https://github.com/zedseven/breeze/commit/49873c3f))
- Rename `sent.rs` to `presentation.rs`. ([12ea5af4](https://github.com/zedseven/breeze/commit/12ea5af4))
- Remove an unnecessary `use` statement. ([f30ca37d](https://github.com/zedseven/breeze/commit/f30ca37d))
- Apply Clippy lints. ([8d8c5aef](https://github.com/zedseven/breeze/commit/8d8c5aef))
- Apply Clippy lints. ([c45f4aae](https://github.com/zedseven/breeze/commit/c45f4aae))
- Apply Clippy lints. ([04fc22a9](https://github.com/zedseven/breeze/commit/04fc22a9))

<!-- generated by git-cliff -->
