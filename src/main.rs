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
    // The cadaclysm libraries it runs on: version, branch, commit and build time, as JSON.
    println!("cadaclysm {}", cadaclysm_sdk::blacksmith::build()?);
    // The nut turns on a turntable, with its controls along the bottom and the frame rate in the
    // bottom-right corner. Every tick and click of the window goes to the frame counter, then the
    // turntable, then the selection; returns when the window is closed.
    let turntable = nut::Turntable::new(&view, &shown.item, [0.0, 0.0, 23.0])?;
    let mut nut = nut::Nut { fps: nut::Fps::new(&view)?, turntable, selection: nut::Selection::new(&view, &nut, &shown.item) };
    view.run_with(|e| nut.on_event(e))?;
    Ok(())
}
