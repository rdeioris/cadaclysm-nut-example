//! The nut and how it is shown, used by both programs: `src/main.rs` on the desktop and
//! `src/web.rs` in the browser.

use cadaclysm_sdk::blacksmith::{Frame, Profile, Solid};
use cadaclysm_sdk::view::{Anchor, Text, View};

#[cfg(target_arch = "wasm32")]
mod web;

/// A hex nut: a hexagon extruded 46 mm, cut down to the part it shares with a turned ring,
/// which bores the hole and chamfers both ends.
pub fn nut() -> cadaclysm_sdk::Result<Solid> {
    // A hexagon 60 mm from the centre to each corner, extruded 46 mm up from the XY plane.
    let hexagon = Profile::regular_polygon([0.0, 0.0], 60.0, 6, 0.0)?;
    let prism = Solid::extrude(&hexagon, &Frame::xy([0.0; 3]), 46.0)?;

    // Half the section of a ring, drawn in the XZ plane (x is the radius, y the height),
    // turned a full circle about the Z axis.
    let section = Profile::polygon(&[
        [30.0, 0.0],
        [45.0, 0.0],
        [50.0, 5.0],
        [70.0, 5.0],
        [70.0, 41.0],
        [50.0, 41.0],
        [45.0, 46.0],
        [30.0, 46.0],
    ])?;
    let ring = Solid::revolve_in_plane(&section, &Frame::xz([0.0; 3]), [0.0, 0.0], [0.0, 1.0], std::f64::consts::TAU)?;

    // What the two share is the nut. 0.05 mm is the tolerance the boolean works at.
    prism.common(&ring, 0.05)
}

/// Steel, in kg per mm³ (7.85 g/cm³). The kernel measures the volume; the material is ours.
const STEEL: f64 = 7.85e-6;

/// Draws the nut in `view` with its figures in the window's corner, and returns a one-line
/// summary. The desktop window and the web page both call this.
pub fn present<'a>(view: &View<'a>, nut: &'a Solid) -> cadaclysm_sdk::Result<String> {
    let mass = nut.mass(0.0)?;
    let faces = nut.faces()?;
    let kg = mass.volume * STEEL;
    let [cx, cy, cz] = mass.centroid;
    let figures = format!(
        "faces {faces}\nvolume {:.0} mm3\nmass {kg:.2} kg (steel, 7.85 g/cm3)\narea {:.0} mm2\ncentroid {cx:.1}, {cy:.1}, {cz:.1} mm",
        mass.volume, mass.area
    );

    view.show_solid(nut, 0.05, None)?;
    // The figures in the window's top-left corner, 16 pixels in, on a dark box.
    let text = Text::overlay(figures).size(16.0).colour("white").box_colour([0.0, 0.0, 0.0, 0.55]).padding(8.0);
    view.show_overlay_text(&text, Anchor::TopLeft, 16.0, 16.0, 0)?;
    view.frame()?;
    Ok(format!("the nut: {faces} faces, {kg:.2} kg of steel"))
}
