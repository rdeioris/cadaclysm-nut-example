//! The desktop program: build the nut, print a few figures, save it as STEP and show it
//! in a window.
//!
//!   cargo run --release

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
