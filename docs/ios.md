# Horizon on iOS (fork)

This fork builds Horizon for iPhone and iPad (iOS 15 or later, arm64 with Metal) as an unsigned
`Horizon.ipa` for sideloading. Like every Horizon build, it holds none of Mojang's files: the
player copies their own prepared resources into the app.

## Build and install

- `.github/workflows/ios.yml` builds on every push to the `ios` branch (GitHub's macOS runner,
  the Xcode 26 SDK) and publishes `Horizon.ipa` and an AltStore-format source,
  `feather-source.json`, to the `ios-latest` prerelease.
- Install with a signing app (Feather, Signulous, AltStore...). Install updates over the app
  without deleting it: iOS keeps its Documents (the resources and the saves) across updates
  with the same bundle ID (`com.sparkkilyy.horizon`) and certificate.
- Microsoft sign-in needs the repository variable `HORIZON_MSA_CLIENT_ID`: an Azure app's
  client ID approved by Mojang ([microsoft-account.md](microsoft-account.md)). Without it the
  game plays offline: single player, LAN by address, offline-mode servers.

## First start

1. Open Horizon once: it makes its folder (Files › On My iPhone › Horizon).
2. Copy the prepared resources (`.local/horizon-client/26.3` of a desktop Horizon install) into
   that folder and name it `resources` (Apple Devices or Finder file sharing, or iCloud Drive and
   the Files app). Delete the `*.log` files in it first.
3. Open Horizon again. Saves, options, logs and crash reports are in the same folder.

## Controls

Bedrock Edition's touch controls with split controls:

| Control | What it does |
|---|---|
| Left of the screen | Floating joystick; pushed fully forward, sprint |
| Anywhere else | Drag to look; tap to use or place (or hit the mob under the crosshair); hold to break |
| Jump, Sneak, » | Jump, sneak (toggles; held to descend while flying or swimming), sprint |
| Use | Shown while the crosshair is on an entity: trade, feed, ride |
| Hotbar | Tap to select, hold to drop; "..." opens the inventory |
| Top right | Perspective, chat, pause (hold pause for the debug screen) |
| Menus | Tap to click (on lift), drag up or down to scroll, drag sideways or hold to drag; X closes; in inventories hold for the right button and the Shift toggle quick-moves |

The system keyboard shows while a text field has focus; its button at the top right or Return
hides it. Bluetooth and MFi controllers work through Apple's GameController framework with
Horizon's Bedrock controller layout ([controllers.md](controllers.md)).

Options › Controls › Touch Controls: look sensitivity, button size and opacity, haptics.
iOS defaults: render distance 6, simulation distance 5, 60 FPS, Auto-Jump on.

## How it fits

- `crates/horizon-ios`: the UIKit and GameController calls (links, the Files app, the pasteboard,
  haptics, alerts, controllers): the one crate with unsafe code for iOS.
- `crates/horizon-account`: the launcher's Microsoft sign-in, for the game itself.
- `crates/horizon-client-input/src/touch.rs`: the controls' layout, fingers and menu gestures
  (unit-tested); `crates/horizon-client/src/game/touch.rs`: drawing them and wiring them to the
  game, the keyboard, saving when the app goes to the background, memory warnings;
  `game/account.rs`: the account screen.

## Limits

- LAN worlds aren't discovered automatically: iOS gives multicast only to apps with Apple's
  restricted entitlement. Join by address (Direct Connect) or with Play with Friends.
- Code mods need a JIT compiler, which iOS doesn't allow: content mods only.
- iOS ends apps that use too much memory: a memory warning lowers the render distance by two.
