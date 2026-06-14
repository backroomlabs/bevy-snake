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

