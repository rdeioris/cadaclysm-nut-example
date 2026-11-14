//! The nut and how it is shown, used by both programs: `src/main.rs` on the desktop and
//! `src/web.rs` in the browser.

use cadaclysm_sdk::blacksmith::{Frame, Profile, Solid};
use cadaclysm_sdk::view::{Anchor, EventKind, Item, Overlay, Pick, Text, View, ViewEvent};

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

/// The nut on show: its item, which a selection colours, and a one-line summary.
pub struct Shown<'v> {
    pub item: Item<'v>,
    pub summary: String,
}

/// Draws the nut in `view` with its figures in the window's corner. The desktop window and
/// the web page both call this.
pub fn present<'v, 'a>(view: &'v View<'a>, nut: &'a Solid) -> cadaclysm_sdk::Result<Shown<'v>> {
    let mass = nut.mass(0.0)?;
    let faces = nut.faces()?;
    let kg = mass.volume * STEEL;
    let [cx, cy, cz] = mass.centroid;
    let figures = format!(
        "faces {faces}\nvolume {:.0} mm3\nmass {kg:.2} kg (steel, 7.85 g/cm3)\narea {:.0} mm2\ncentroid {cx:.1}, {cy:.1}, {cz:.1} mm",
        mass.volume, mass.area
    );

    let item = view.show_solid(nut, 0.05, None)?;
    // The figures in the window's top-left corner, 16 pixels in, on a dark box.
    let text = Text::overlay(figures).size(16.0).colour("white").box_colour([0.0, 0.0, 0.0, 0.55]).padding(8.0);
    view.show_overlay_text(&text, Anchor::TopLeft, 16.0, 16.0, 0)?;
    view.frame()?;
    Ok(Shown { item, summary: format!("the nut: {faces} faces, {kg:.2} kg of steel") })
}

/// Face `face` of `nut` in words: the surface it lies on, its area, and the centre and outward
/// normal of the workplane the kernel lays on it.
pub fn face_info(nut: &Solid, face: u32) -> cadaclysm_sdk::Result<String> {
    let kind = nut.face_kind(face)?;
    let area = nut.face_sheet(face)?.mass(0.0)?.area;
    let frame = nut.face_frame(face)?;
    let ([x, y, z], [nx, ny, nz]) = (frame.origin(), frame.z());
    Ok(format!(
        "face {face} of {}\n{kind}\narea {area:.0} mm2\ncentre {x:.1}, {y:.1}, {z:.1} mm\nnormal {nx:.2}, {ny:.2}, {nz:.2}",
        nut.faces()?
    ))
}

/// The face the last click chose: orange on the nut, its details in an overlay in the
/// window's top-right corner. `View::run_with` hands every event to [`on_event`](Self::on_event).
pub struct Selection<'v, 'a> {
    view: &'v View<'a>,
    nut: &'a Solid,
    item: &'v Item<'v>,
    info: Option<Overlay<'v>>,
}

impl<'v, 'a> Selection<'v, 'a> {
    pub fn new(view: &'v View<'a>, nut: &'a Solid, item: &'v Item<'v>) -> Self {
        Selection { view, nut, item, info: None }
    }

    /// A click on a face of the nut selects it; a click on nothing clears the selection; a
    /// click on an overlay changes nothing. Always keeps the view running.
    pub fn on_event(&mut self, event: &ViewEvent) -> bool {
        if let EventKind::Pick(pick) = &event.kind {
            if let Err(e) = self.pick(*pick) {
                report(&e.to_string());
            }
        }
        true
    }

    fn pick(&mut self, pick: Option<Pick>) -> cadaclysm_sdk::Result<()> {
        match pick {
            Some(p) if p.overlay => Ok(()),
            Some(p) if p.item == self.item.id() => self.select(p.face),
            _ => self.clear(),
        }
    }

    fn select(&mut self, face: u32) -> cadaclysm_sdk::Result<()> {
        self.item.set_face_colours(&[(face, "orange")])?;
        let text = face_info(self.nut, face)?;
        match &self.info {
            Some(info) => info.set_text(&text)?,
            None => {
                let style = Text::overlay(text).size(16.0).colour("white").box_colour([0.0, 0.0, 0.0, 0.55]).padding(8.0);
                self.info = Some(self.view.show_overlay_text(&style, Anchor::TopRight, 16.0, 16.0, 0)?);
            }
        }
        Ok(())
    }

    fn clear(&mut self) -> cadaclysm_sdk::Result<()> {
        self.item.clear_face_colours()?;
        match self.info.take() {
            Some(info) => info.remove(),
            None => Ok(()),
        }
    }
}

/// An error a click ran into: the browser's console, or standard error on the desktop.
fn report(message: &str) {
    #[cfg(target_arch = "wasm32")]
    web_sys::console::error_1(&message.into());
    #[cfg(not(target_arch = "wasm32"))]
    eprintln!("{message}");
}
