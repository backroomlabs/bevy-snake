# Bevy Snake

A simple 10×10 Snake game built with [Bevy](https://bevyengine.org/), targeting **Desktop**, **Web (WASM)**, and **Android**.

## Controls

| Platform | Input |
|---|---|
| Desktop / Web | Arrow keys or WASD |
| Android / touch | Swipe in any direction |
| Restart | R / Space / Enter / Tap |

---

## Running locally

### Desktop

```bash
cargo run
```

### Web (requires [trunk](https://trunkrs.dev/))

```bash
# Install trunk once
cargo install trunk

# Add the WASM target once
rustup target add wasm32-unknown-unknown

# Serve with live-reload
trunk serve

# Or produce a production build in dist/
trunk build --release
```

Then open `http://localhost:8080`.

### Android (requires Android SDK + NDK)

```bash
# Install cargo-apk once
cargo install cargo-apk

# Add Android targets once
rustup target add aarch64-linux-android armv7-linux-androideabi

# Set NDK path (adjust to your installation)
export ANDROID_NDK_ROOT=$ANDROID_HOME/ndk/27.0.12077973

# Build a debug APK
cargo apk build --lib

# Install directly to a connected device / emulator
adb install target/debug/apk/bevy-tutor1.apk
```

> **Note:** Android builds require the `android_game_activity` Bevy feature, which is
> automatically enabled by the `[target.'cfg(target_os = "android")'.dependencies]`
> entry in `Cargo.toml`. You do **not** need to pass `--features` manually.

---

## CI / CD

Two GitHub Actions workflows ship with this repo:

### `web.yml` — WASM build + GitHub Pages deploy

| Trigger | What happens |
|---|---|
| Every push / PR | Compiles to WASM with `trunk build --release`, uploads `dist/` as an artifact |
| Push to `main` | Additionally deploys to **GitHub Pages** |

**One-time setup:**  
Go to *Settings → Pages → Source* and choose **"GitHub Actions"**.  
After the first successful push to `main`, your game is live at  
`https://<username>.github.io/<repo>/`.

### `android.yml` — APK build (+ optional release signing)

| Trigger | What happens |
|---|---|
| Every push / PR | Builds a **debug APK** with `cargo apk build --lib`, uploads it as an artifact |
| Tag push `v*` | Builds a **release APK** and signs it with a keystore |

**One-time setup for signed releases:**  
Create a keystore and add these repository secrets:

| Secret | Value |
|---|---|
| `KEYSTORE_BASE64` | `base64 -w0 your.keystore` |
| `KEYSTORE_PASSWORD` | keystore password |
| `KEY_ALIAS` | key alias |
| `KEY_PASSWORD` | key password |

Then push a version tag to trigger the signed build:
```bash
git tag v1.0.0 && git push origin v1.0.0
```

---

## Project structure

```
src/
  main.rs          # Desktop + WASM binary entry (calls run())
  lib.rs           # All game logic + Android entry (#[bevy_main])
android/
  AndroidManifest.xml   # Android activity declaration
.github/workflows/
  web.yml          # WASM CI / GitHub Pages
  android.yml      # Android APK CI
index.html         # Trunk entry point for web builds
Cargo.toml         # Both [lib] (cdylib) and [[bin]] targets
```

## How cross-platform works

```
                    ┌─────────────┐
                    │   src/lib.rs │  ← all game code lives here
                    │  + #[bevy_main] │
                    └──────┬──────┘
                           │
          ┌────────────────┼─────────────────┐
          ▼                ▼                 ▼
    src/main.rs       trunk build       cargo apk build
    (desktop binary)  (wasm32 binary)   (cdylib → .so → APK)
```

- **Desktop / WASM**: `cargo` / `trunk` compile the `[[bin]]` target (`main.rs`), which calls `bevy_tutor1::run()` from the lib.
- **Android**: `cargo apk` compiles the `[lib]` target as a `cdylib`. The `#[bevy_main]` macro generates an `android_main` C symbol that Android's `GameActivity` calls at startup.
- **Touch input**: swipe detection is built in via Bevy's `TouchInput` events — no extra crates needed.
