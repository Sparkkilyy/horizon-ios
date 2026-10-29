<p align="center">
  <img src="docs/media/brand/icon-1024.png" alt="Horizon" width="128">
</p>

<h1 align="center">Horizon for iOS</h1>

<p align="center">
  Minecraft: Java Edition 26.3, rewritten in Rust, on iPhone and iPad, with Bedrock-style touch controls.
</p>

<p align="center">
  <a href="https://github.com/Sparkkilyy/horizon-ios/releases/tag/ios-latest"><b>Download the IPA</b></a> ·
  <a href="docs/ios.md"><b>iOS notes</b></a> ·
  <a href="https://github.com/horizon-rust/horizon"><b>Upstream Horizon</b></a>
</p>

This is an unofficial fork of [Horizon](https://github.com/horizon-rust/horizon), a native Rust
rewrite of Minecraft: Java Edition 26.3 that reads the same worlds, follows the same rules and
generates the same terrain as the Java game. The fork adds an iOS build: Metal rendering, touch
controls modelled on Bedrock Edition, touch-friendly menus, the system keyboard, controllers,
haptics, and Microsoft sign-in inside the game (iOS has no launcher). Everything else, the game
itself, comes from upstream Horizon; credit for it goes to its authors.

> Not an official Minecraft product, and not approved by or associated with Mojang or Microsoft.
> Neither this fork nor Horizon contains any Minecraft code, assets or data: you need your own
> legally obtained copy of Minecraft: Java Edition.

## Requirements

- An iPhone or iPad on iOS 15 or later (tested on an iPhone 12, iOS 27).
- A way to sideload apps: Feather, Signulous, AltStore, SideStore or similar, with your own
  signing certificate. The IPA is unsigned.
- A computer (Windows, macOS or Linux) with the
  [Horizon launcher](https://playhorizon.dev) and a Microsoft account that owns Minecraft: Java
  Edition, to prepare the game's resources once (below).

## Install

1. Download `Horizon.ipa` from the
   [`ios-latest` release](https://github.com/Sparkkilyy/horizon-ios/releases/tag/ios-latest).
   It is rebuilt automatically on every change. Feather and AltStore can also add the source
   `feather-source.json` from the same release.
2. Sign and install it with your sideloading app.
3. Open Horizon once. It creates its folder in **Files › On My iPhone › Horizon** and tells you
   the resources are missing.

**Updating:** install the new IPA *over* the existing app, without deleting it first, using the
same certificate. iOS then keeps the app's folder: your resources, worlds and settings.
Deleting the app deletes them too, so back up your `saves` folder first if you ever need to.

## Setting up the resources

Horizon draws and plays with the files from your own copy of Minecraft, prepared by the desktop
launcher. Until the app can download them itself, copy them over from a computer once:

1. **Prepare them on the computer.** Install the [Horizon launcher](https://playhorizon.dev),
   sign in, and press **Play** once. The launcher downloads Minecraft 26.3 for your account and
   prepares it. You can close the game once it reaches the title screen.
2. **Find the prepared folder.** It is the `26.3` folder inside the launcher's version folder:

   | System | Folder |
   |---|---|
   | Windows | `%APPDATA%\horizon-launcher\versions\<version>\.local\horizon-client\26.3` |
   | macOS / Linux | `.local/horizon-client/26.3` inside the launcher's `versions/<version>` folder |

   `<version>` is the Horizon version you played, for example `nightly-2026-10-28-…`.
3. **Clean it.** Delete any `*.log` files inside it. They are the computer's game logs and don't
   belong on the phone.
4. **Copy it to the phone** and rename it to `resources`, so that you end up with
   **On My iPhone › Horizon › resources**. Any of these works:
   - **Windows:** the Apple Devices app (or iTunes) → your iPhone → Files → Horizon → drag the
     folder in.
   - **Mac:** Finder → your iPhone → Files → Horizon → drag the folder in.
   - **iCloud Drive:** copy the folder to iCloud Drive, then move it into the Horizon folder with
     the Files app.
5. **Open Horizon again.** It loads to the title screen.

The resources must match the game's version. If a new Horizon build needs newer ones, the app
says so: repeat these steps with the matching launcher version.

## Controls

Bedrock-style touch controls with split controls:

| Control | What it does |
|---|---|
| Left of the screen | Floating joystick; push it fully forward to sprint |
| Anywhere else | Drag to look; tap to use or place; hold to break |
| Jump, Sneak, » | Jump, sneak (hold to go down while flying or swimming), sprint |
| Hotbar | Tap to select, hold to drop; **…** opens the inventory |
| Top right | Perspective, chat, pause (hold pause for the debug screen) |
| Menus | Tap to click, drag to scroll, hold for right click; **X** closes |

Change look sensitivity, button size and opacity, and haptics under **Options › Controls › Touch
Controls**. Bluetooth and MFi controllers work too, with Horizon's Bedrock controller layout.

## Multiplayer

- **Works without an account:** single player, LAN worlds by address (Direct Connect), Play with
  Friends codes, and offline-mode servers.
- **Online-mode servers** need Microsoft sign-in under **Options › Horizon Settings › Microsoft
  Account**. A build signs in only when it was made with an approved Microsoft app ID
  ([docs/microsoft-account.md](docs/microsoft-account.md)); until then the screen says sign-in
  isn't configured.

## Limits

- LAN worlds aren't found automatically: iOS allows multicast only for apps with Apple's restricted
  entitlement. Join by address instead.
- Code mods need a JIT compiler, which iOS doesn't allow: only content mods, resource packs and
  data packs work.
- iOS closes apps that use too much memory. Horizon starts at render distance 6 and lowers it when
  iOS warns about memory.

## Privacy

Your game files stay on your devices. The app's folder holds your resources, worlds, settings and
logs. The Microsoft sign-in is kept in the app's private Library folder, not in the folder the
Files app shows. Never upload your `resources` folder or logs anywhere public. This repository's
pre-commit check (`tools/check_public_tree.py`) keeps them out of commits.

## Building

The IPA is built by GitHub Actions on macOS ([`.github/workflows/ios.yml`](.github/workflows/ios.yml))
for every push to the `ios` branch. No Mac is needed to work on the fork. See
[docs/ios.md](docs/ios.md) for how the iOS code fits into Horizon. For the game itself, its
documentation and contributing, see [upstream Horizon](https://github.com/horizon-rust/horizon)
and [CONTRIBUTING.md](CONTRIBUTING.md).

## License

MIT, like Horizon: see [LICENSE](LICENSE) and [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
