//! The browser program. The page hands its canvas to a Web Worker, which loads the
//! cadaclysm web package, builds the nut and draws it there, so the page never freezes
//! while the kernel works.

use cadaclysm_sdk::view::{View, ViewOptions};
use cadaclysm_sdk::web::page::{self, App};
use cadaclysm_sdk::web::{worker, LoadOptions};
use wasm_bindgen::prelude::*;

/// Runs on the page (index.html calls it): start the worker and ask it about the nut.
#[wasm_bindgen]
pub fn page_main() -> Result<(), JsValue> {
    let document = web_sys::window().and_then(|w| w.document()).expect("a page");
    let canvas: web_sys::HtmlCanvasElement = document
        .get_element_by_id("nut")
        .ok_or("no canvas called nut")?
        .dyn_into()
        .map_err(|_| "#nut isn't a canvas")?;

    let worker = page::spawn(&canvas, App { glue: "./app/nut.js", entry: "worker_main", package: "./pkg/index.js" })?;
    let status = document.get_element_by_id("status");
    worker.on_message(move |data| {
        if let Some(status) = &status {
            status.set_text_content(data.as_string().as_deref());
        }
    });
    // Sent at once, while the worker is still loading: it is kept until the worker listens.
    worker.post(&JsValue::from_str("figures?"))?;
    Ok(())
}

/// Runs in the worker: load the package, draw the nut on the canvas, answer the page.
#[wasm_bindgen]
pub async fn worker_main() -> Result<(), JsValue> {
    let worker = worker::start(LoadOptions::new("./pkg/index.js")).await?;
    let nut = crate::nut()?;
    let view = View::on_canvas(worker.canvas(), &ViewOptions::default()).await?;
    let shown = crate::present(&view, &nut)?;
    let summary = shown.summary.clone();

    let reply = worker.clone();
    worker.on_message(move |data| {
        if data.as_string().as_deref() == Some("figures?") {
            let _ = reply.post(&JsValue::from_str(&summary));
        }
    });
    // Hands over the waiting messages and the page's drag, wheel and pinch, gives each click
    // to the selection, and keeps the view (and the nut) alive while the page is open.
    let mut selection = crate::Selection::new(&view, &nut, &shown.item);
    view.run_with(|e| selection.on_event(e)).await?;
    Ok(())
}
