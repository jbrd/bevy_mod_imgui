# bevy_mod_imgui

![Crates.io](https://img.shields.io/crates/v/bevy_mod_imgui)
![Crates.io](https://img.shields.io/crates/l/bevy_mod_imgui)
![Build Status](https://github.com/jbrd/bevy_mod_imgui/actions/workflows/rust.yml/badge.svg)
![docs.rs](https://img.shields.io/docsrs/bevy_mod_imgui)

A Dear ImGui integration for the Bevy game engine.

![bevy_mod_imgui screenshot](media/screenshot.png)

## Current Status

This repository is actively maintained and updated when new versions of Bevy are released. New feature
requests are also welcome, although this project remains relatively low priority, so it may take some
time for these to be honoured. Contributions welcome - please do start an issue prior to working on
a feature so that it can be discussed before spending time on it.

This crate is not related to any official Bevy organisation repository in any way.

## Compatibility Table

|`bevy_mod_imgui`|`bevy`  |`wgpu`  |`imgui` |`imgui-wgpu`      |
|----------------|--------|--------|--------|------------------|
| 0.8.*          | 0.17.* | 26.*   | 0.12.* | 0.24.0 (bundled) |
| 0.7.*          | 0.16.* | 24.*   | 0.12.* | 0.24.0 (bundled) |
| 0.6.*          | 0.15.* | 23.*   | 0.12.* | 0.24.0 (bundled) |
| 0.5.*          | 0.14.* | 0.20.* | 0.11.* | 0.24.0 (bundled) |
| 0.4.*          | 0.14.* | 0.20.* | 0.11.* | 0.24.0 (bundled) |
| 0.3.*          | 0.13.* | 0.19.* | 0.11.* | 0.24.0 (bundled) |
| 0.2.*          | 0.12.* | 0.17.1 | 0.11.* | 0.24.*           |
| 0.1.*          | 0.11.* | 0.16.* | 0.11.* | 0.23.*           |

## Examples

The following examples are provided:

* `custom-texture` - to demonstrate how to display a Bevy texture in an ImGui window
* `empty` - to demonstrate that an empty draw list is handled gracefully (bug regression example)
* `hello-world` - to demonstrate basic ImGui functionality (via its demo window)
* `hello-world-postupdate` - to demonstrate emitting ImGui from the PostUpdate stage
* `minimal` - to demonstrate the most minimal example of setting up the plug-in
* `render-to-texture` - to demonstrate rendering a Bevy scene to a texture and displaying the result on an ImGui window


## Changelog

* `0.8.0` - Update to wgpu `26.0`, Bevy `0.17.0`.
* `0.7.2` - Fix backend renderer to support ImGui 1.86+ modals
* `0.7.1` - Fix for crash in imgui-wpu-rs when the draw list is empty
* `0.7.0` - Update to wgpu `24.0`, Bevy `0.16.0`. Improved safety / stability of texture management
* `0.6.0` - Update to imgui-rs `0.12.0`, wgpu `23.0`, Bevy `0.15.0`
* `0.5.1` - Various threading and safety fixes. Fix crash when plugin used with Bevy `multi_threaded` feature
* `0.5.0` - Add support for drawing Bevy textures in ImGui windows. Fix assert on exit introduced in Bevy 0.14.1
* `0.4.0` - Updated dependencies for Bevy `0.14.0`. Improved handling of display scale changes.
* `0.3.0` - Updated dependencies for Bevy `0.13.0` with bundled `imgui-wgpu-rs`.
* `0.2.1` - Fix Issue #20 - unchecked window lookup which could cause panic during exit
* `0.2.0` - Updated dependencies for Bevy `0.12.0`
* `0.1.1` - Fix Issue #20 - unchecked window lookup which could cause panic during exit (backported from `0.2.1`)
* `0.1.0` - Initial crate publish

## Contributors

* James Bird (@jbrd)
* @nhlest
* @PJB3005
* Marius Metzger (@CrushedPixel)

## Acknowledgements

This crate builds upon the fantastic work of the following projects:

  * [Dear ImGui](https://github.com/ocornut/imgui)
  * [imgui-rs](https://github.com/imgui-rs/imgui-rs)
  * [imgui-wgpu-rs](https://github.com/Yatekii/imgui-wgpu-rs)
  * [Bevy](https://github.com/bevyengine/bevy)

## License

All code in this repository is permissively dual-licensed under:

* MIT License - [LICENSE-MIT](LICENSE-MIT)
* Apache License, Version 2.0 - [LICENSE-APACHE](LICENSE-APACHE)

## Bundled Software License Notices

### imgui-wgpu-rs

This software contains portions of code derived from [imgui-wgpu-rs](https://github.com/Yatekii/imgui-wgpu-rs/tree/master).
https://github.com/Yatekii/imgui-wgpu-rs/tree/master
Licensed under the Apache License

Copyright (c) 2019 Steven Wittens

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.