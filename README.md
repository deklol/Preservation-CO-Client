<p align="center">
  <a href="https://conquer.dek.cx"><img src="docs/images/logo.png" alt="Preservation Conquer" width="720"></a>
</p>

<h1 align="center">Preservation CO Client</h1>

<p align="center">An offline Conquer Online skeleton client. Written in Rust.</p>

<p align="center">
  <a href="https://github.com/deklol/Preservation-CO-Client/releases/latest"><img src="https://img.shields.io/github/v/release/deklol/Preservation-CO-Client?label=Latest%20release" alt="Latest release"></a>
  <img src="https://img.shields.io/badge/Platform-Windows%20x64-0078D4" alt="Platform: Windows x64">
  <img src="https://img.shields.io/badge/Language-Rust-CE422B?logo=rust" alt="Language: Rust">
  <img src="https://img.shields.io/badge/Renderer-wgpu-6E56CF" alt="Renderer: wgpu">
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-Apache%202.0-blue" alt="License: Apache 2.0"></a>
</p>

<p align="center">
  <a href="https://discord.gg/CvKPXEHYRY">Discord</a> ·
  <a href="https://conquer.dek.cx">Preservation Conquer</a> ·
  <a href="https://dek.cx">dek.cx</a> ·
  <a href="https://x.com/digitalm1nd">@digitalm1nd</a> ·
  <a href="https://github.com/deklol">GitHub</a>
</p>

<video src="https://github.com/deklol/Preservation-CO-Client/releases/download/v0.1.1/skeleton-demo.mp4" controls width="960"></video>

[Watch the gameplay video](https://i.dek.cx/8xcp.mp4)

## What's here

This is a small part of my Preservation Conquer client, shared for people who want to learn or build their own. It loads an empty Twin City locally as `dek`.

- Map and character rendering, equipment, shadows and nameplates
- Walking, running, jumping and collision
- Original minimap with a player marker, zoom and expand controls
- Readers for the original client files

**This is the skeleton, not the full client.** There is no server connection, combat, skill system, effects or game windows. Game assets are not included.

Want to play the full client? **[Try it on the Preservation CO server](https://conquer.dek.cx).**

## Build and run

You need a **Conquer Online 5065 installation**, Rust 1.95 or newer, and Visual Studio C++ Build Tools with the Windows SDK. Tested on Windows x64.

[Download the 5065 client files](https://mega.nz/#!lVwQyJBZ!mIe5uRJu0SZGJQloky0l0kfbxRkT8UmrzYU7Z4Rb7Hg) and extract them into `C:\Conquer5065`. That folder must directly contain `ini`, `map`, `data.wdf` and `c3.wdf`, not another folder containing them. Keep these files separate from this source project.

```sh
git clone https://github.com/deklol/Preservation-CO-Client.git
cd Preservation-CO-Client
cargo run --release --locked -- --assets "C:\Conquer5065"
```

If you extract somewhere else, replace `C:\Conquer5065` in the command with that folder's full path.

## Controls

| Input | Action |
| --- | --- |
| Left click or hold | Move |
| Ctrl + click or hold | Jump |
| `/` | Toggle run/walk |
| Shift + click | Turn |
| Space | Stop after the current step or jump |
| Escape | Exit |
| Minimap buttons | Toggle crop/full-map view and expand/collapse |

Edit `character.ini` and rebuild to change the local character. Use `--help` for launch options. Layered PUX maps are not supported.

## Developers and preservationists

Working on a Conquer client, researching the old game, or interested in helping preserve it? **[Join the Discord](https://discord.gg/CvKPXEHYRY).**

I share more detailed explanations, source snippets and progress there. You can also get involved in beta testing the full Preservation Conquer client. Find me as **_dek**.

## License and credit

By [@digitalm1nd](https://x.com/digitalm1nd) / **_dek** on Discord, for Preservation Conquer.

The source code is licensed under [Apache 2.0](LICENSE). You can use, modify and distribute it, including commercially. When redistributing, include the license, retain applicable source attribution notices, mark changed files and reproduce the relevant credits from [NOTICE](NOTICE), as required by the license.

Credit in your README or credits screen is appreciated, but Apache 2.0 also allows the NOTICE file or accompanying documentation. Game assets and the project logo are not covered by this source code license. Dependencies keep their own licenses.

## Tests

```sh
cargo test --workspace --locked
cargo run --release --locked -- --assets "C:\Conquer5065" --check-assets
```

Run `./tools/package-source.ps1` to create a source ZIP in `dist/`. No game assets or build files are packaged.
