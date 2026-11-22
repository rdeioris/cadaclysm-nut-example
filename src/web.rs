//! The browser program. The page hands its canvas to a Web Worker, which loads the
//! cadaclysm web package, builds the nut and draws it there, so the page never freezes
//! while the kernel works.

use cadaclysm_sdk::view::{Anchor, Text, View, ViewOptions};
use cadaclysm_sdk::web::page::{self, App};
use cadaclysm_sdk::web::{worker, LoadOptions};
use wasm_bindgen::prelude::*;

/// Runs on the page (index.html calls it): start the worker, ask it about the nut and hand its
/// answers to `show`.
#[wasm_bindgen]
pub fn page_main(show: js_sys::Function) -> Result<(), JsValue> {
    let document = web_sys::window().and_then(|w| w.document()).expect("a page");
    let canvas: web_sys::HtmlCanvasElement = document
        .get_element_by_id("nut")
        .ok_or("no canvas called nut")?
        .dyn_into()
        .map_err(|_| "#nut isn't a canvas")?;

    let worker = page::spawn(&canvas, App { glue: "./app/nut.js", entry: "worker_main", package: "./pkg/index.js" })?;
    worker.on_message(move |data| {
        let _ = show.call1(&JsValue::NULL, &data);
    });
    // Sent at once, while the worker is still loading: it is kept until the worker listens.
    worker.post(&JsValue::from_str("figures?"))?;
    Ok(())
}

/// Runs in the worker: load the package, draw the blank nut at once, cut its thread, then
/// draw the nut and answer the page.
#[wasm_bindgen]
pub async fn worker_main() -> Result<(), JsValue> {
    let worker = worker::start(LoadOptions::new("./pkg/index.js")).await?;
    let blank = crate::blank()?;
    // Cut below, once the blank is on the canvas; declared here so it outlives the view.
    let nut;
    let view = View::on_canvas(worker.canvas(), &ViewOptions::default()).await?;

    // The blank first: it takes a moment to build, the thread several seconds to cut. It is
    // drawn now and the worker lets the browser show the frame before the cut starts, which
    // keeps the worker busy until it is done.
    let waiting = view.show_solid(&blank, 0.05, None)?;
    let note = Text::overlay("cutting the thread…").size(16.0).colour("white").box_colour([0.0, 0.0, 0.0, 0.55]).padding(8.0);
    let note = view.show_overlay_text(&note, Anchor::TopLeft, 16.0, 16.0, 0)?;
    view.frame()?;
    view.draw()?;
    worker.post(&JsValue::from_str("cutting the thread…"))?;
    let_the_browser_draw().await?;

    nut = crate::threaded(&blank)?;
    waiting.remove()?;
    note.remove()?;
    let shown = crate::present(&view, &nut)?;
    // The answer to "figures?": the summary, and the cadaclysm build as the kernel gives it
    // (JSON text), for the page to show.
    let figures = js_sys::Object::new();
    js_sys::Reflect::set(&figures, &"summary".into(), &shown.summary.as_str().into())?;
    js_sys::Reflect::set(&figures, &"build".into(), &cadaclysm_sdk::blacksmith::build()?.into())?;

    let reply = worker.clone();
    worker.on_message(move |data| {
        if data.as_string().as_deref() == Some("figures?") {
            let _ = reply.post(&figures);
        }
    });
    // Hands over the waiting messages and the page's drag, wheel and pinch, gives each tick and
    // click to the frame counter, then the turntable, then the selection, and keeps the view
    // (and the nut) alive while the page is open.
    let turntable = crate::Turntable::new(&view, &shown.item, [0.0, 0.0, 23.0])?;
    let mut nut = crate::Nut { fps: crate::Fps::new(&view)?, turntable, selection: crate::Selection::new(&view, &nut, &shown.item) };
    view.run_with(|e| nut.on_event(e)).await?;
    Ok(())
}

/// Hands the worker back to the browser for a moment, so the frame just drawn reaches the
/// screen and the page's status line its message before the worker is busy again.
async fn let_the_browser_draw() -> Result<(), JsValue> {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        let scope: web_sys::WorkerGlobalScope = js_sys::global().unchecked_into();
        let _ = scope.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 50);
    });
    wasm_bindgen_futures::JsFuture::from(promise).await?;
    Ok(())
}
