# bevy_mod_imgui

A Dear ImGui integration for the Bevy game engine.

![Build Status](https://github.com/jbrd/bevy_mod_imgui/actions/workflows/rust.yml/badge.svg)

![bevy_mod_imgui screenshot](media/screenshot.png)

## Current Status

Note that this repository should be treated as **experimental** at the moment. I threw this together very
quickly to get something up and running for some personal projects.

This crate has only been tested on Windows (DX12 and Vulcan).

This crate is not related to any official Bevy organisation repository in any way.

## Feedback Saught

If you're interested in playing with this crate, I'd be keen to hear your thoughts and how you get on with it (sharing your feedback with stars or issues would be very useful!). I'd be particularly interested in hearing whether anyone has any success with this crate on different platforms so I can keep track of this. I'm also happy to receive source code contributions, please do start an issue if you are considering this.

If there is a large enough appetite for this crate, I may consider taking it further...

## Compatibility Table

|`bevy_mod_imgui`|`bevy`  |`wgpu`  |`imgui` |`imgui-wgpu`|
|----------------|--------|--------|--------|------------|
| 0.1.0          | 0.11.* | 0.16.* | 0.11.* | 0.23.*     |

## Contributors

* James Bird (@jbrd)
* @nhlest

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