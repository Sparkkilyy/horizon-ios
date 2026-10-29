//! The touch controls on iOS (`horizon_client::touch`): in the world, fingers drive a virtual
//! controller and turn the view, and the controls are drawn over the HUD. In menus the first
//! finger stays the mouse (game.rs `touch_as_mouse`).
use super::Game;
use horizon_client::{
    gamepad::{Button, PadState},
    hud::{HudBatch, rgba},
    touch::{STICK_RADIUS, TouchButton, TouchLayout},
};
use std::time::Instant;
use winit::event::{Touch, TouchPhase};

impl Game<'_> {
    /// No touch controls: a menu, a dialog or no world.
    fn touch_paused(&self) -> bool {
        self.pad_in_menu() || self.dialog_open() || self.player.is_none()
    }

    /// A touch for the controls: false when it is a menu's, for the mouse. A finger that went
    /// down in the world stays the controls' until it is lifted.
    pub(super) fn touch_event(&mut self, touch: &Touch) -> bool {
        let owned = self.pad.touch.owns(touch.id);
        if !owned && (touch.phase != TouchPhase::Started || self.touch_paused()) {
            return false;
        }
        let Some(batch) = self.gui_scale() else {
            return false;
        };
        let at = [
            (touch.location.x / f64::from(batch.scale)) as f32,
            (touch.location.y / f64::from(batch.scale)) as f32,
        ];
        let now = Instant::now();
        let controls = &mut self.pad.touch;
        controls.layout = TouchLayout::new(batch.width, batch.height);
        match touch.phase {
            TouchPhase::Started => controls.start(touch.id, at, now),
            TouchPhase::Moved => controls.moved(touch.id, at),
            TouchPhase::Ended | TouchPhase::Cancelled => controls.end(touch.id, at, now),
        }
        self.last_input = now;
        true
    }

    /// Once per loop iteration, before the pads are read (pad.rs `poll_pads`): the controls'
    /// virtual pad. The view's turn goes to the mouse's accumulated movement, as a grabbed
    /// pointer's would, and a tapped hotbar slot is selected as its number key selects it.
    pub(super) fn touch_frame(&mut self) -> PadState {
        if self.touch_paused() {
            self.pad.touch.release_all();
            return PadState::default();
        }
        let entity = self.entity_targeted();
        self.pad.touch.use_shown = entity;
        let frame = self.pad.touch.frame(Instant::now());
        if frame.look != [0.0; 2]
            && let Some(batch) = self.gui_scale()
        {
            // A screen pixel turns the view as a mouse pixel does (0.15 degrees at the default
            // sensitivity): a drag across half an iPhone's screen is about half a turn.
            self.mouse.add_motion(
                f64::from(frame.look[0] * batch.scale),
                f64::from(frame.look[1] * batch.scale),
            );
        }
        if let Some(slot) = frame.hotbar {
            self.selected = slot;
        }
        let mut pad = frame.pad;
        // Each lasts one poll; the pad holds a press back until a tick sees it.
        if frame.drop {
            pad.set(Button::East, true);
        }
        if frame.tap {
            // Bedrock's tap: hit the mob under the crosshair, else use the item or place.
            pad.triggers[usize::from(entity)] = 1.0;
        }
        pad
    }

    /// The controls over the HUD while playing: translucent buttons, and the joystick where the
    /// thumb is (or where it rests).
    pub(super) fn draw_touch_controls(&self, batch: &mut HudBatch) {
        if self.touch_paused() || self.ui.hide_gui {
            return;
        }
        let atlas = &self.gui.atlas;
        let controls = &self.pad.touch;
        let layout = TouchLayout::new(batch.width, batch.height);
        for button in TouchButton::ALL {
            if !controls.shown(button) {
                continue;
            }
            let r = layout.button(button);
            let held = controls.held(button);
            batch.fill(atlas, r.x, r.y, r.w, r.h, rgba(0, 0, 0, if held { 150 } else { 80 }));
            let edge = rgba(255, 255, 255, if held { 200 } else { 90 });
            batch.fill(atlas, r.x, r.y, r.w, 1.0, edge);
            batch.fill(atlas, r.x, r.y + r.h - 1.0, r.w, 1.0, edge);
            batch.fill(atlas, r.x, r.y, 1.0, r.h, edge);
            batch.fill(atlas, r.x + r.w - 1.0, r.y, 1.0, r.h, edge);
            let label = match button {
                TouchButton::Jump => "Jump",
                TouchButton::Sneak => "Sneak",
                TouchButton::Inventory => "...",
                TouchButton::Perspective => "F5",
                TouchButton::Pause => "II",
                TouchButton::Use => "Use",
            };
            let w = HudBatch::text_width(atlas, label);
            batch.text(
                atlas,
                label,
                (r.x + (r.w - w) / 2.0).round(),
                (r.y + (r.h - 8.0) / 2.0).round(),
                rgba(255, 255, 255, 230),
                true,
            );
        }
        let (origin, tilt) = controls.stick().unwrap_or((
            [layout.inset + STICK_RADIUS + 16.0, layout.height - STICK_RADIUS - 30.0],
            [0.0, 0.0],
        ));
        let active = controls.stick().is_some();
        disc(batch, atlas, origin, STICK_RADIUS, rgba(0, 0, 0, if active { 90 } else { 50 }));
        let knob = [
            origin[0] + tilt[0] * STICK_RADIUS,
            origin[1] - tilt[1] * STICK_RADIUS,
        ];
        disc(batch, atlas, knob, 10.0, rgba(255, 255, 255, if active { 170 } else { 70 }));
    }
}

/// A filled circle, as a fan of quads.
fn disc(
    batch: &mut HudBatch,
    atlas: &horizon_client::hud::HudAtlas,
    [cx, cy]: [f32; 2],
    radius: f32,
    color: u32,
) {
    const SEGMENTS: usize = 24;
    let point = |i: usize| {
        let a = i as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
        [cx + radius * a.cos(), cy + radius * a.sin()]
    };
    for i in 0..SEGMENTS {
        let (a, b) = (point(i), point(i + 1));
        batch.fill_quad(atlas, [[cx, cy], a, b, b], color);
    }
}
