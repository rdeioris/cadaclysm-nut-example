//! The desktop program: build the nut, save it as STEP and show it in a window.
//!
//!   cargo run --release

use cadaclysm_sdk::blacksmith::Unit;
use cadaclysm_sdk::view::{self, View, ViewOptions};

fn main() -> cadaclysm_sdk::Result<()> {
    let nut = nut::nut()?;

    nut.step("nut.step", None, Unit::Millimetre)?;
    println!("saved nut.step");

    // A window of its own: drag to orbit, wheel to zoom, click a face to select it.
    view::use_wgpu()?;
    let view = View::new(&ViewOptions { title: Some("cadaclysm nut".into()), ..ViewOptions::default() })?;
    let shown = nut::present(&view, &nut)?;
    println!("{}", shown.summary);
    // Hands every event of the window to the selection; returns when the window is closed.
    let mut selection = nut::Selection::new(&view, &nut, &shown.item);
    view.run_with(|e| selection.on_event(e))?;
    Ok(())
}
