//! The nut, built once and used by both programs: `src/main.rs` on the desktop and
//! `src/web.rs` in the browser.

use cadaclysm_sdk::blacksmith::{Frame, Profile, Solid};

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
