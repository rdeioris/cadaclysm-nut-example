//! The browser program. The page hands its canvas to a Web Worker, which loads the
//! cadaclysm web package, builds the nut and draws it there, so the page never freezes
//! while the kernel works.

use cadaclysm_sdk::web::page::{self, App};
use cadaclysm_sdk::web::{worker, LoadOptions, ShowOptions, Viewer};
use wasm_bindgen::prelude::*;

/// Runs on the page (index.html calls it): start the worker and ask it for the nut.
#[wasm_bindgen]
pub fn page_main() -> Result<(), JsValue> {
    let document = web_sys::window().and_then(|w| w.document()).expect("a page");
    let canvas: web_sys::HtmlCanvasElement = document
        .get_element_by_id("nut")
        .ok_or("no canvas called nut")?
        .dyn_into()
        .map_err(|_| "#nut isn't a canvas")?;

    let worker = page::spawn(&canvas, App { glue: "./app/nut.js", entry: "worker_main", package: "./pkg/index.js" })?;
    worker.on_message(move |data| {
        if let Some(status) = document.get_element_by_id("status") {
            status.set_text_content(data.as_string().as_deref());
        }
    });
    // Sent at once, while the worker is still loading: it is kept until the worker is ready.
    worker.post(&JsValue::from_str("build"))?;
    Ok(())
}

/// Runs in the worker: load the package, take the canvas, draw the nut when asked.
#[wasm_bindgen]
pub async fn worker_main() -> Result<(), JsValue> {
    let worker = worker::start(LoadOptions::new("./pkg/index.js")).await?;
    let viewer = Viewer::create(worker.canvas(), Default::default()).await?;
    viewer.receive_controls()?; // drag, wheel, pinch and resize, forwarded by the page
    viewer.start()?;

    let reply = worker.clone();
    worker.on_message(move |data| {
        if data.as_string().as_deref() == Some("build") {
            let text = match crate::nut() {
                Ok(nut) => {
                    let _ = viewer.show_solid(&nut, ShowOptions::default());
                    let _ = viewer.frame();
                    format!("the nut: {} faces", nut.faces().unwrap_or(0))
                }
                Err(e) => e.to_string(),
            };
            let _ = reply.post(&JsValue::from_str(&text));
        }
    });
    Ok(())
}
