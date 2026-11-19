//! The nut and how it is shown, used by both programs: `src/main.rs` on the desktop and
//! `src/web.rs` in the browser.

use cadaclysm_sdk::blacksmith::{Animation, Frame, InterpolatorMode, Profile, Solid, ThreadOptions};
use cadaclysm_sdk::view::{Anchor, AnimationBinding, EventKind, Item, Overlay, OverlayStyle, Pick, Text, View, ViewEvent};

#[cfg(target_arch = "wasm32")]
mod web;

/// A hex nut: a hexagon extruded 46 mm, cut down to the part it shares with a turned ring,
/// which bores the hole and chamfers both ends, and a screw thread cut into the bore.
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
    let blank = prism.common(&ring, 0.05)?;

    // The bore is the one cylinder face of the blank whose middle lies 30 mm from the axis.
    let mut bores = Vec::new();
    for face in 0..blank.faces()? {
        // face_ref: the surface's kind (cylinder 1), then a point on the face's middle.
        let r = blank.face_ref(face)?;
        if r[0] == 1.0 && (r[1].hypot(r[2]) - 30.0).abs() < 1e-6 {
            bores.push(face);
        }
    }
    assert_eq!(bores.len(), 1, "the nut's bore: expected one cylinder face 30 mm from the axis, found {bores:?}");

    // An internal thread: the explicit form, a pitch of 6 mm (ISO coarse for a 60 mm bore) with
    // the bore taken as the basic minor diameter, along the whole face, cut at the same 0.05 mm
    // as the boolean above.
    let options = ThreadOptions { pitch: Some(6.0), tolerance: Some(0.05), ..ThreadOptions::default() };
    let (nut, _report) = blank.thread(bores[0], None, &options)?;
    Ok(nut)
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

/// The turn takes this long, in seconds.
const TURN_SECONDS: f64 = 6.0;
/// The seek bar is this many segments, each a click target.
const SEGMENTS: usize = 24;
/// The controls sit along the bottom, in overlay units from the bottom-left corner.
const MARGIN: f32 = 16.0;
const BUTTON_W: f32 = 64.0;
const SEGMENT_W: f32 = 12.0;
const GAP: f32 = 8.0;
const BAR_X: f32 = MARGIN + BUTTON_W + GAP;
const LOOP_X: f32 = BAR_X + SEGMENTS as f32 * SEGMENT_W + GAP;
const READOUT_X: f32 = LOOP_X + BUTTON_W + GAP;

/// A control's text: white on the same dark box as the figures.
fn control_text(text: &str) -> Text {
    Text::overlay(text).size(16.0).colour("white").box_colour([0.0, 0.0, 0.0, 0.55]).padding(6.0)
}

/// The nut turning once about its centre on a looping timeline, and the controls along the
/// bottom that drive it: a play/pause button, a seek bar (orange up to the playhead; a click
/// seeks to that segment), a loop toggle and a readout of the time and the angle.
pub struct Turntable<'v, 'a> {
    view: &'v View<'a>,
    animation: Animation,
    button: Overlay<'v>,
    loop_toggle: Overlay<'v>,
    readout: Overlay<'v>,
    segments: Vec<Overlay<'v>>,
    /// How many segments are orange now.
    filled: usize,
    looping: bool,
    playing: bool,
}

impl<'v, 'a> Turntable<'v, 'a> {
    /// Plays a turn of `item` about `pivot`, the nut's centre, and shows the controls.
    pub fn new(view: &'v View<'a>, item: &Item<'v>, pivot: [f64; 3]) -> cadaclysm_sdk::Result<Self> {
        let mut animation = Animation::new()?;
        // A rotate track turns whatever is bound to "nut": 0 to 360 degrees about the Z axis.
        animation.rotate("nut", &[(0.0, 0.0), (TURN_SECONDS, 360.0)], pivot, [0.0, 0.0, 1.0], InterpolatorMode::Linear)?;
        // A value track moves nothing: the readout asks it for the angle at the playhead.
        animation.value("angle", &[(0.0, [0.0]), (TURN_SECONDS, [360.0])], InterpolatorMode::Linear)?;
        view.play_animation(&animation, &[AnimationBinding { target: "nut", item: item.id(), node: None }])?;
        view.set_loop(true)?;
        view.play(1.0)?;

        let button = view.show_overlay_text(&control_text("pause"), Anchor::BottomLeft, MARGIN, MARGIN, 0)?;
        let segments = (0..SEGMENTS).map(|k| segment(view, k, false)).collect::<cadaclysm_sdk::Result<Vec<_>>>()?;
        let loop_toggle = view.show_overlay_text(&control_text("loop on"), Anchor::BottomLeft, LOOP_X, MARGIN, 0)?;
        let readout = view.show_overlay_text(&control_text(&readout_text(&animation, 0.0)?), Anchor::BottomLeft, READOUT_X, MARGIN, 0)?;
        Ok(Turntable { view, animation, button, loop_toggle, readout, segments, filled: 0, looping: true, playing: true })
    }

    /// A click on a control or a timeline tick: true when the event was the turntable's.
    pub fn on_event(&mut self, event: &ViewEvent) -> cadaclysm_sdk::Result<bool> {
        match &event.kind {
            EventKind::Timeline => {
                self.tick()?;
                Ok(true)
            }
            EventKind::Pick(Some(p)) if p.overlay => self.click(p.item),
            _ => Ok(false),
        }
    }

    fn click(&mut self, id: u32) -> cadaclysm_sdk::Result<bool> {
        if id == self.button.id() {
            if self.view.playing() {
                self.view.pause()?;
            } else {
                self.view.play(1.0)?;
            }
            self.show_playing(self.view.playing())?;
        } else if id == self.loop_toggle.id() {
            self.looping = !self.looping;
            self.view.set_loop(self.looping)?;
            self.loop_toggle.set_text(if self.looping { "loop on" } else { "loop off" })?;
        } else if let Some(k) = self.segments.iter().position(|s| s.id() == id) {
            // The middle of the segment's slice of the turn.
            self.view.seek((k as f64 + 0.5) / SEGMENTS as f64 * TURN_SECONDS)?;
        } else {
            return Ok(false);
        }
        Ok(true)
    }

    fn show_playing(&mut self, playing: bool) -> cadaclysm_sdk::Result<()> {
        if playing != self.playing {
            self.playing = playing;
            self.button.set_text(if playing { "pause" } else { "play" })?;
        }
        Ok(())
    }

    /// The readout, the bar and the button at the timeline's clock.
    fn tick(&mut self) -> cadaclysm_sdk::Result<()> {
        let t = self.view.time();
        self.readout.set_text(&readout_text(&self.animation, t)?)?;
        // A segment is orange once the playhead reaches its middle.
        let filled = (t / TURN_SECONDS * SEGMENTS as f64 + 0.5).floor().clamp(0.0, SEGMENTS as f64) as usize;
        for k in self.filled.min(filled)..self.filled.max(filled) {
            // An overlay shape keeps the fill it was shown with, so a new colour is a new shape.
            let old = std::mem::replace(&mut self.segments[k], segment(self.view, k, k < filled)?);
            old.remove()?;
        }
        self.filled = filled;
        self.show_playing(self.view.playing())
    }
}

/// Segment `k` of the seek bar: a grey rectangle, orange when `filled`.
fn segment<'v>(view: &'v View<'_>, k: usize, filled: bool) -> cadaclysm_sdk::Result<Overlay<'v>> {
    let (w, h) = ((SEGMENT_W - 2.0) as f64, 28.0);
    let rectangle = Profile::polygon(&[[0.0, 0.0], [w, 0.0], [w, h], [0.0, h]])?;
    let style = if filled { OverlayStyle::default().fill("orange") } else { OverlayStyle::default().fill([0.35; 3]) };
    view.show_overlay(&rectangle, Anchor::BottomLeft, BAR_X + k as f32 * SEGMENT_W, MARGIN, &style.stroke("none"))
}

/// `3.1 / 6.0 s  187 deg`: the time, and the angle the value track holds then.
fn readout_text(animation: &Animation, t: f64) -> cadaclysm_sdk::Result<String> {
    let angle = animation.value_at(1, t)?[0];
    Ok(format!("{t:.1} / {TURN_SECONDS:.1} s  {angle:.0} deg"))
}

/// What `run_with` hands each event to: the turntable first, then the selection.
pub struct Nut<'v, 'a> {
    pub turntable: Turntable<'v, 'a>,
    pub selection: Selection<'v, 'a>,
}

impl Nut<'_, '_> {
    /// The turntable's event, else the selection's. Always keeps the view running.
    pub fn on_event(&mut self, event: &ViewEvent) -> bool {
        match self.turntable.on_event(event) {
            Ok(true) => true,
            Ok(false) => self.selection.on_event(event),
            Err(e) => {
                report(&e.to_string());
                true
            }
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
