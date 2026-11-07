# cadaclysm nut example

A hex nut modelled in Rust with [cadaclysm](https://cadaclysm.blitter.studio), then
shown in two places from the same code: a desktop window and a web page.

| On the desktop | In the browser |
|---|---|
| ![The nut in a desktop window](screenshots/desktop.png) | ![The nut in a web page](screenshots/browser.png) |

The nut is an exact B-rep solid, not a mesh: a hexagon prism intersected with a turned
ring, giving 13 faces. You can save it as STEP and open it in any CAD program.

This README walks through the example one step at a time. The code is short:
[`src/lib.rs`](src/lib.rs) builds the nut, [`src/main.rs`](src/main.rs) is the desktop
program and [`src/web.rs`](src/web.rs) is the browser one.

## What you need

- Rust (stable), from [rustup](https://rustup.rs).
- For the desktop: nothing else. When the `cadaclysm-sdk` crate builds, it downloads the
  cadaclysm libraries for your platform (Windows x64, macOS 11+, Linux x64 and arm64).
- For the browser:
  - the wasm target: `rustup target add wasm32-unknown-unknown`
  - `wasm-bindgen-cli`, at the same version as the `wasm-bindgen` in `Cargo.lock`
    (0.2.130 here): `cargo install wasm-bindgen-cli --version 0.2.130`
  - the cadaclysm web package, `cadaclysm-0.13.0-web-pro.zip`, from the
    [SDK's releases](https://github.com/rdeioris/cadaclysm-sdk/releases/tag/v0.13.0)
  - any static web server. The steps below use Python's.

## Step 1: the crate

[`Cargo.toml`](Cargo.toml) describes one crate that builds two programs:

```toml
[lib]
crate-type = ["cdylib", "rlib"]

[[bin]]
name = "desktop"
path = "src/main.rs"

[dependencies]
cadaclysm-sdk = "0.13.1"

[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen = "0.2.100"
wasm-bindgen-futures = "0.4.50"
web-sys = { version = "0.3.77", features = ["Document", "Element", "HtmlCanvasElement", "Window"] }
```

- The library holds the nut. The desktop program links it as an `rlib`. The browser
  loads it as a `cdylib`, a `.wasm` file.
- `cadaclysm-sdk` is the only dependency on the desktop.
- The three wasm crates are needed only when building for the browser.

## Step 2: model the nut

[`src/lib.rs`](src/lib.rs) builds the nut in three moves:

```rust
use cadaclysm_sdk::blacksmith::{Frame, Profile, Solid};

pub fn nut() -> cadaclysm_sdk::Result<Solid> {
    // A hexagon 60 mm from the centre to each corner, extruded 46 mm up from the XY plane.
    let hexagon = Profile::regular_polygon([0.0, 0.0], 60.0, 6, 0.0)?;
    let prism = Solid::extrude(&hexagon, &Frame::xy([0.0; 3]), 46.0)?;

    // Half the section of a ring, drawn in the XZ plane (x is the radius, y the height),
    // turned a full circle about the Z axis.
    let section = Profile::polygon(&[
        [30.0, 0.0], [45.0, 0.0], [50.0, 5.0], [70.0, 5.0],
        [70.0, 41.0], [50.0, 41.0], [45.0, 46.0], [30.0, 46.0],
    ])?;
    let ring = Solid::revolve_in_plane(&section, &Frame::xz([0.0; 3]), [0.0, 0.0], [0.0, 1.0], std::f64::consts::TAU)?;

    // What the two share is the nut. 0.05 mm is the tolerance the boolean works at.
    prism.common(&ring, 0.05)
}
```

1. **The prism.** `Profile::regular_polygon` draws a flat hexagon, and `Solid::extrude`
   pushes it 46 mm up along the frame's Z axis.
2. **The ring.** The section is half of a ring's cross-section: its inner edge at radius
   30 mm is the hole, and its sloped corners at the top and bottom become the chamfers.
   `Solid::revolve_in_plane` turns it a full circle about the Z axis.
3. **The nut.** `common` keeps only the volume that the prism and the ring share: the
   ring bores the hole and chamfers both faces, and the prism keeps its six flat sides.

Every call returns a `Result`. A bad input, such as a polygon with two sides, comes back
as an error with the kernel's own explanation.

## Step 3: show it on the desktop

[`src/main.rs`](src/main.rs) builds the nut, prints a few figures, saves a STEP file and
opens a window:

```rust
use cadaclysm_sdk::blacksmith::Unit;
use cadaclysm_sdk::view;

fn main() -> cadaclysm_sdk::Result<()> {
    let nut = nut::nut()?;

    let mass = nut.mass(0.0)?;
    println!("faces: {}", nut.faces()?);
    println!("volume: {:.1} mm3", mass.volume);

    nut.step("nut.step", None, Unit::Millimetre)?;
    println!("saved nut.step");

    // A window of its own (drag to orbit, wheel to zoom); returns when it is closed.
    view::use_wgpu()?;
    nut.show(None)?;
    Ok(())
}
```

Run it:

```sh
cargo run --release
```

The first build takes a minute because it downloads the libraries. Then:

```text
faces: 13
volume: 277596.4 mm3
cadaclysm: unlicensed -- running without a production license; licenses at https://cadaclysm.blitter.studio/license
saved nut.step
```

The window opens with the nut framed. Drag to orbit and use the wheel to zoom. The
program ends when you close the window.

![The nut in a desktop window](screenshots/desktop.png)

- **`view::use_wgpu()`** picks the windowed viewer. Without it, `show` draws the nut
  straight into your terminal (sixel, kitty graphics or coloured blocks), which also
  works over SSH.
- **The notice** is normal: without a licence the libraries run in full and say so.
  See [The licence](#the-licence) below.

## Step 4: the browser program

A browser can't load native libraries. On `wasm32`, `cadaclysm-sdk` instead calls the
cadaclysm web package, a wasm module that runs in the same page. The modelling code
doesn't change: `nut()` from step 2 runs as it is.

The kernel blocks the thread it runs on, so it shouldn't run on the page's own thread or
the page would freeze. [`src/web.rs`](src/web.rs) has two halves: one on the page and one
in a Web Worker.

**On the page**, `page_main` starts the worker and asks it for the nut:

```rust
#[wasm_bindgen]
pub fn page_main() -> Result<(), JsValue> {
    // ... find the <canvas id="nut"> ...
    let worker = page::spawn(&canvas, App { glue: "./app/nut.js", entry: "worker_main", package: "./pkg/index.js" })?;
    worker.on_message(move |data| { /* show the worker's reply in the status line */ });
    worker.post(&JsValue::from_str("build"))?;
    Ok(())
}
```

`page::spawn` does the setup a worker needs:

- sizes the canvas to its on-screen size in device pixels;
- starts a worker that runs this same wasm, beginning at `worker_main`;
- hands the canvas over to the worker;
- forwards drag, wheel, pinch and resize events to it.

The `"build"` message is posted at once, while the worker is still loading. It is kept
and delivered once the worker is ready.

**In the worker**, `worker_main` loads the package, makes a viewer on the canvas and
draws the nut when asked:

```rust
#[wasm_bindgen]
pub async fn worker_main() -> Result<(), JsValue> {
    let worker = worker::start(LoadOptions::new("./pkg/index.js")).await?;
    let viewer = Viewer::create(worker.canvas(), Default::default()).await?;
    viewer.receive_controls()?;
    viewer.start()?;

    let reply = worker.clone();
    worker.on_message(move |data| {
        if data.as_string().as_deref() == Some("build") {
            // crate::nut(), then viewer.show_solid(&nut, ShowOptions::default()) and viewer.frame()
            // ... and reply.post(...) with "the nut: 13 faces"
        }
    });
    Ok(())
}
```

`show_solid` hands the solid to the package's viewer as it is. Nothing is copied: the
viewer meshes the exact solid itself.

**The page**, [`web/index.html`](web/index.html), is a canvas, a status line and four
lines of script:

```html
<div id="status">loading…</div>
<canvas id="nut"></canvas>
<script type="module">
import init, { page_main } from "./app/nut.js";
await init();
page_main();
</script>
```

`page_main` is called from the page rather than marked `#[wasm_bindgen(start)]`. The
worker runs the same wasm, and a start function would run there too.

## Step 5: build and run in the browser

1. Build the library for wasm. `--lib` leaves out the desktop program:

   ```sh
   cargo build --lib --release --target wasm32-unknown-unknown
   ```

   Nothing is downloaded this time. The browser build uses the web package instead of
   the native libraries.

2. Generate the JavaScript glue into `web/app`:

   ```sh
   wasm-bindgen --target web --out-dir web/app target/wasm32-unknown-unknown/release/nut.wasm
   ```

3. Unzip `cadaclysm-0.13.0-web-pro.zip` and copy its `pkg` folder to `web/pkg`. The
   folder should now hold:

   ```text
   web/
     index.html
     app/   nut.js, nut_bg.wasm, snippets/   (from wasm-bindgen)
     pkg/   index.js, view.js, cadaclysm_bg.wasm, ...   (from the zip)
   ```

4. Serve the folder and open it:

   ```sh
   python -m http.server 8000 --directory web
   ```

   Then go to <http://localhost:8000>. The page must come from a server: browsers don't
   run wasm modules or workers from `file://`.

The status line says "loading…", then "the nut: 13 faces", and the nut appears. Drag to
orbit, use the wheel or a pinch to zoom.

![The nut in a web page](screenshots/browser.png)

Your app's own wasm is under half a megabyte (about 90 KB gzipped). Most of the download
is the web package's wasm, about 5.5 MB gzipped.

**If the page stays on "loading…"**, open the browser's console:

- *"Failed to load module script ... MIME type"*: the server sent `.js` as plain text.
  On Windows, Python's server takes its types from the registry. Use another static
  server, or fix the `.js` entry under `HKEY_CLASSES_ROOT\.js` to `text/javascript`.
- *A wasm-bindgen schema version error*: the `wasm-bindgen` CLI doesn't match the
  version in `Cargo.lock`. Install the matching one.

## The licence

Development, testing and evaluation are free. Without a licence everything runs in full
and prints a notice on each open and export. A product, server or website licence
([cadaclysm.blitter.studio/license](https://cadaclysm.blitter.studio/license)) removes it.

- **Desktop:** put `cadaclysm.lic` beside the executable, or set `CADACLYSM_LICENSE`.
- **Browser:** pass it when loading the package, as its text or a URL to fetch it from:

  ```rust
  worker::start(LoadOptions::new("./pkg/index.js").licence("./cadaclysm.lic")).await?;
  ```

## Going further

- The `blacksmith` module has the rest of the modelling kernel: fillets, shells, sweeps,
  lofts, NURBS curves and surfaces, measurements, and STEP and 3DM output. See the
  [Rust documentation](https://cadaclysm.blitter.studio/docs/rust.html).
- In the browser, `Solid::step_text` gives the STEP file as a string to offer as a download.
  Calls that write a file directly refuse, because a page has no file system.
- In the browser, reading CAD files and healing aren't in the Rust crate yet; they work on
  the desktop.

The example uses `cadaclysm-sdk` 0.13.1 with the 0.13.0 web package.
