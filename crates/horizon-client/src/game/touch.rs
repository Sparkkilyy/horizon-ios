//! iOS: the touch controls in the world (`horizon_client::touch`), fingers in menus as the mouse
//! (tap, drag to scroll, drag, hold), the close and Shift buttons over menus, the system keyboard
//! for text fields, and saving when the app goes to the background.
use super::{Game, Screen};
use horizon_client::{
    gamepad::{Button, PadState},
    hud::{HudAtlas, HudBatch, rgba},
    osk::Output,
    touch::{MenuAction, Rect, STICK_RADIUS, TouchButton, TouchLayout},
};
use std::time::Instant;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalPosition,
    event::{
        DeviceId, ElementState, KeyEvent, MouseButton, MouseScrollDelta, Touch, TouchPhase,
        WindowEvent,
    },
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
};

/// GUI pixels of finger travel per scrolled line (a mouse wheel's notch).
const PIXELS_PER_LINE: f32 = 10.0;

impl Game<'_> {
    /// No touch controls: a menu, a dialog or no world.
    fn touch_paused(&self) -> bool {
        self.pad_in_menu() || self.dialog_open() || self.player.is_none()
    }

    /// An inventory, where fingers move items: drags never scroll, a hold is the right button.
    fn touch_slots(&self) -> bool {
        self.title.is_none() && matches!(self.screen, Screen::Inventory | Screen::Creative)
    }

    /// The close button shows: a screen over a world that Escape closes.
    fn touch_close_shown(&self) -> bool {
        self.title.is_none()
            && self.player.is_some()
            && (self.pad_in_menu() || self.dialog_open())
            && (self.screen != Screen::Dead || self.ui.death_confirm.is_some())
    }

    fn touch_gui(&self, touch: &Touch) -> Option<([f32; 2], TouchLayout)> {
        let batch = self.gui_scale()?;
        let at = [
            (touch.location.x / f64::from(batch.scale)) as f32,
            (touch.location.y / f64::from(batch.scale)) as f32,
        ];
        Some((at, TouchLayout::new(batch.width, batch.height)))
    }

    /// Every touch while the game runs (game.rs `App::window_event`): the world's controls take
    /// fingers that go down while playing, until lifted; the others are the menus' mouse.
    pub(super) fn handle_touch(&mut self, event_loop: &ActiveEventLoop, touch: Touch) {
        let Some((at, layout)) = self.touch_gui(&touch) else {
            return;
        };
        let now = Instant::now();
        self.last_input = now;
        self.pad.touch_device = Some(touch.device_id);
        let playing = !self.touch_paused();
        let controls = &mut self.pad.touch;
        if controls.owns(touch.id) || (touch.phase == TouchPhase::Started && playing) {
            controls.layout = layout;
            match touch.phase {
                TouchPhase::Started => controls.start(touch.id, at, now),
                TouchPhase::Moved => controls.moved(touch.id, at),
                TouchPhase::Ended | TouchPhase::Cancelled => controls.end(touch.id, at, now),
            }
            return;
        }
        if self.pad.menu_ignored == Some(touch.id) {
            if matches!(touch.phase, TouchPhase::Ended | TouchPhase::Cancelled) {
                self.pad.menu_ignored = None;
            }
            return;
        }
        let slots = self.touch_slots();
        let actions = match touch.phase {
            TouchPhase::Started if !self.pad.menu_touch.owns(touch.id) => {
                if self.pad.keyboard_shown && layout.keyboard_hide().contains(at) {
                    self.pad.menu_ignored = Some(touch.id);
                    self.pad.keyboard_dismissed = true;
                    return;
                }
                if self.touch_close_shown() && layout.close().contains(at) {
                    self.pad.menu_ignored = Some(touch.id);
                    if let Err(e) = self.touch_back() {
                        self.error = Some(e);
                    }
                    return;
                }
                if slots && layout.shift().contains(at) {
                    self.pad.menu_ignored = Some(touch.id);
                    self.pad.touch_shift = !self.pad.touch_shift;
                    return;
                }
                self.pad.menu_touch.start(touch.id, at, now)
            }
            TouchPhase::Started => Vec::new(),
            TouchPhase::Moved => self.pad.menu_touch.moved(touch.id, at, slots),
            TouchPhase::Ended | TouchPhase::Cancelled => {
                // A tap may have focused a text field the keyboard went away from: show it again
                // if one has focus after it.
                self.pad.keyboard_shown = false;
                self.pad.keyboard_dismissed = false;
                self.pad.menu_touch.end(touch.id, at)
            }
        };
        self.menu_actions(event_loop, actions);
    }

    /// The menu finger's actions as the mouse's events, through the game's own handling.
    fn menu_actions(&mut self, event_loop: &ActiveEventLoop, actions: Vec<MenuAction>) {
        let (Some(id), Some(batch)) = (self.window.as_ref().map(|w| w.id()), self.gui_scale())
        else {
            return;
        };
        let device_id = self.pad.touch_device.unwrap_or_else(DeviceId::dummy);
        for action in actions {
            let event = match action {
                MenuAction::Move([x, y]) => WindowEvent::CursorMoved {
                    device_id,
                    position: PhysicalPosition::new(
                        f64::from(x * batch.scale),
                        f64::from(y * batch.scale),
                    ),
                },
                MenuAction::Press { right } => {
                    // The Shift toggle: a left press in an inventory is a shift-click.
                    if self.pad.touch_shift && !right && self.touch_slots() {
                        self.pad.shift_added = (
                            self.keys.insert(KeyCode::ShiftLeft),
                            self.physical_keys.insert(KeyCode::ShiftLeft),
                        );
                    }
                    mouse_button(device_id, right, ElementState::Pressed)
                }
                MenuAction::Release { right } => {
                    self.window_event(
                        event_loop,
                        id,
                        mouse_button(device_id, right, ElementState::Released),
                    );
                    let (keys, physical) = std::mem::take(&mut self.pad.shift_added);
                    if keys {
                        self.keys.remove(&KeyCode::ShiftLeft);
                    }
                    if physical {
                        self.physical_keys.remove(&KeyCode::ShiftLeft);
                    }
                    continue;
                }
                MenuAction::Scroll(dy) => WindowEvent::MouseWheel {
                    device_id,
                    delta: MouseScrollDelta::LineDelta(0.0, dy / PIXELS_PER_LINE),
                    phase: TouchPhase::Moved,
                },
            };
            self.window_event(event_loop, id, event);
        }
    }

    /// Once per loop iteration (game.rs `App::about_to_wait`): a menu finger held still presses,
    /// and the keyboard shows while a text field has focus.
    pub(super) fn touch_poll(&mut self, event_loop: &ActiveEventLoop) {
        if !self.touch_paused() {
            self.pad.menu_touch.cancel();
        }
        if !self.touch_slots() {
            self.pad.touch_shift = false;
        }
        let actions = self
            .pad
            .menu_touch
            .poll(Instant::now(), self.touch_slots());
        self.menu_actions(event_loop, actions);
        let typing = self.text_input_active();
        if !typing {
            self.pad.keyboard_dismissed = false;
        }
        let typing = typing && !self.pad.keyboard_dismissed;
        if typing != self.pad.keyboard_shown
            && let Some(window) = &self.window
        {
            window.set_ime_allowed(typing);
            self.pad.keyboard_shown = typing;
        }
    }

    /// A text field has focus: the pad's list (chat, signs, books, searches, the anvil), the
    /// title screens' fields, World Options' fields, the command block and the mod browser.
    fn text_input_active(&mut self) -> bool {
        self.pad_typing()
            || self.title.as_mut().is_some_and(|t| t.has_caret_field())
            || (self.screen == Screen::WorldOptions && self.world_options_typing())
            || (self.screen == Screen::CommandBlock && self.title.is_none())
            || self.mod_browser_typing()
    }

    /// The system keyboard's keys (game.rs `Game::window_event`). iOS sends characters with no
    /// physical key, which most fields' key handlers skip, and Return as the text "\n": they go
    /// to the focused field as the on-screen keyboard's output does. Backspace arrives as a key.
    pub(super) fn ios_typed(&mut self, event: &KeyEvent) -> bool {
        if !matches!(event.physical_key, PhysicalKey::Unidentified(_)) {
            return false;
        }
        if event.state == ElementState::Pressed
            && let Some(text) = event.text.as_deref()
        {
            if text == "\n" || text == "\r" {
                self.pad_osk_output(Output::Enter);
                // Return is also how the keyboard goes away: a field that keeps focus (a world's
                // name) would otherwise keep it over the buttons under it.
                self.pad.keyboard_dismissed = true;
            } else {
                self.pad_osk_output(Output::Text(text.to_owned()));
            }
        }
        true
    }

    /// The close button: what Escape does on the screen that is open.
    fn touch_back(&mut self) -> horizon_client::Result<()> {
        if self.screen == Screen::Dead && self.ui.death_confirm.is_some() {
            self.death_confirm_answer(false)
        } else if self.link_prompt.is_some() {
            self.answer_link_prompt(false);
            Ok(())
        } else if self.server_packs.prompt.is_some() {
            self.answer_pack_prompt(false)
        } else if self.dialog_open() {
            self.dialog_key(KeyCode::Escape, None, None)
        } else {
            self.pad_back()
        }
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

    /// The app went to the background, where iOS may end it without warning: pause as vanilla
    /// does when the window loses focus, stop the world's clock, and save the world and the
    /// options now.
    pub(super) fn ios_suspend(&mut self) {
        let started = Instant::now();
        if self.screen == Screen::Playing && self.player.is_some() && self.title.is_none() {
            self.screen = Screen::Paused;
            self.capture(false);
        }
        self.pad.touch.release_all();
        self.pad.menu_touch.cancel();
        self.pause = self.pauses_game();
        if let Some(server) = &self.integrated {
            server.set_paused(self.pause);
        }
        let saved = if let Some(server) = &self.integrated {
            server
                .call(|g| g.save_game().map_err(|e| e.to_string()))
                .unwrap_or_else(|| Err("the integrated server didn't answer".into()))
        } else if self.remote.is_none() && self.title.is_none() && self.pending_save.is_none() {
            self.save_game().map_err(|e| e.to_string())
        } else {
            Ok(())
        };
        if let Err(e) = saved {
            log::warn!("Horizon: saving the world in the background failed: {e}");
        }
        self.settings.save(&Self::options_path());
        log::info!(
            "Horizon: saved for the background in {} ms",
            started.elapsed().as_millis()
        );
    }

    /// Back from the background: no ticks to catch up, and frames again.
    pub(super) fn ios_resume(&mut self) {
        self.previous = Instant::now();
        self.pad.keyboard_shown = false;
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    /// The controls over the HUD while playing, after Bedrock's: translucent grey tiles with
    /// icons, the joystick where the thumb is (or where it rests).
    pub(super) fn draw_touch_controls(&self, batch: &mut HudBatch) {
        if self.ui.hide_gui {
            return;
        }
        let atlas = &self.gui.atlas;
        let layout = TouchLayout::new(batch.width, batch.height);
        if self.pad.keyboard_shown {
            let hide = layout.keyboard_hide();
            tile(batch, atlas, hide, false);
            keyboard_down(batch, atlas, hide);
        }
        if self.touch_paused() {
            if self.touch_close_shown() {
                let close = layout.close();
                tile(batch, atlas, close, false);
                cross(batch, atlas, close);
                if self.touch_slots() {
                    let shift = layout.shift();
                    tile(batch, atlas, shift, self.pad.touch_shift);
                    arrow(batch, atlas, shift, true, true);
                }
            }
            return;
        }
        let controls = &self.pad.touch;
        let sneaking = self.keys.contains(&KeyCode::ShiftLeft);
        for button in TouchButton::ALL {
            if !controls.shown(button) {
                continue;
            }
            let r = layout.button(button);
            let on = controls.held(button) || (button == TouchButton::Sneak && sneaking);
            tile(batch, atlas, r, on);
            match button {
                TouchButton::Jump => arrow(batch, atlas, r, true, true),
                TouchButton::Sneak => arrow(batch, atlas, r, false, true),
                TouchButton::Sprint => chevrons(batch, atlas, r),
                TouchButton::Inventory => dots(batch, atlas, r),
                TouchButton::Chat => bubble(batch, atlas, r),
                TouchButton::Perspective => eye(batch, atlas, r),
                TouchButton::Pause => bars(batch, atlas, r),
                TouchButton::Use => {
                    let w = HudBatch::text_width(atlas, "Use");
                    batch.text(
                        atlas,
                        "Use",
                        (r.x + (r.w - w) / 2.0).round(),
                        (r.y + (r.h - 8.0) / 2.0).round(),
                        WHITE,
                        true,
                    );
                }
            }
        }
        // The joystick: a rounded square base and a square knob, as Bedrock draws it.
        let (origin, tilt) = controls
            .stick()
            .unwrap_or((layout.joystick_rest(), [0.0, 0.0]));
        let active = controls.stick().is_some();
        let half = STICK_RADIUS + 4.0;
        rounded(
            batch,
            atlas,
            Rect::new(origin[0] - half, origin[1] - half, 2.0 * half, 2.0 * half),
            rgba(30, 30, 30, if active { 110 } else { 70 }),
        );
        let knob = 18.0;
        let (kx, ky) = (
            origin[0] + tilt[0] * STICK_RADIUS - knob / 2.0,
            origin[1] - tilt[1] * STICK_RADIUS - knob / 2.0,
        );
        rounded(
            batch,
            atlas,
            Rect::new(kx, ky, knob, knob),
            rgba(160, 160, 160, if active { 200 } else { 120 }),
        );
    }
}

const WHITE: u32 = 0xe6ff_ffff;

fn mouse_button(device_id: DeviceId, right: bool, state: ElementState) -> WindowEvent {
    WindowEvent::MouseInput {
        device_id,
        state,
        button: if right {
            MouseButton::Right
        } else {
            MouseButton::Left
        },
    }
}

/// A Bedrock button: a translucent dark grey square with a light top-left edge and a dark
/// bottom-right one, lighter while held.
fn tile(batch: &mut HudBatch, atlas: &HudAtlas, r: Rect, on: bool) {
    let fill = if on {
        rgba(120, 120, 120, 170)
    } else {
        rgba(40, 40, 40, 110)
    };
    batch.fill(atlas, r.x, r.y, r.w, r.h, fill);
    let light = rgba(255, 255, 255, if on { 110 } else { 60 });
    let dark = rgba(0, 0, 0, 90);
    batch.fill(atlas, r.x, r.y, r.w, 1.0, light);
    batch.fill(atlas, r.x, r.y + 1.0, 1.0, r.h - 1.0, light);
    batch.fill(atlas, r.x + 1.0, r.y + r.h - 1.0, r.w - 1.0, 1.0, dark);
    batch.fill(atlas, r.x + r.w - 1.0, r.y + 1.0, 1.0, r.h - 2.0, dark);
}

/// A square with its corners cut: Bedrock's rounded joystick shapes at GUI scale.
fn rounded(batch: &mut HudBatch, atlas: &HudAtlas, r: Rect, color: u32) {
    let c = (r.w.min(r.h) / 6.0).round().max(1.0);
    batch.fill(atlas, r.x + c, r.y, r.w - 2.0 * c, r.h, color);
    batch.fill(atlas, r.x, r.y + c, c, r.h - 2.0 * c, color);
    batch.fill(atlas, r.x + r.w - c, r.y + c, c, r.h - 2.0 * c, color);
}

fn triangle(batch: &mut HudBatch, atlas: &HudAtlas, a: [f32; 2], b: [f32; 2], c: [f32; 2]) {
    batch.fill_quad(atlas, [a, b, c, c], WHITE);
}

/// Bedrock's jump and sneak icons: an arrow up (or down) over a bar.
fn arrow(batch: &mut HudBatch, atlas: &HudAtlas, r: Rect, up: bool, bar: bool) {
    let (cx, cy) = (r.x + r.w / 2.0, r.y + r.h / 2.0);
    let s = r.w.min(r.h) * 0.22;
    let dir = if up { -1.0 } else { 1.0 };
    let tip = cy + dir * s * 1.4;
    let base = cy - dir * s * 0.1;
    triangle(
        batch,
        atlas,
        [cx, tip],
        [cx - s * 1.2, base],
        [cx + s * 1.2, base],
    );
    let stem = s * 0.45;
    let (top, bottom) = if up {
        (base, cy + s * 0.9)
    } else {
        (cy - s * 0.9, base)
    };
    batch.fill(atlas, cx - stem, top, 2.0 * stem, bottom - top, WHITE);
    if bar {
        let y = if up { cy + s * 1.3 } else { cy - s * 1.3 - 2.0 };
        batch.fill(atlas, cx - s * 1.2, y, s * 2.4, 2.0, WHITE);
    }
}

/// Sprint: a double chevron, ».
fn chevrons(batch: &mut HudBatch, atlas: &HudAtlas, r: Rect) {
    let (cx, cy) = (r.x + r.w / 2.0, r.y + r.h / 2.0);
    let s = r.w.min(r.h) * 0.18;
    for dx in [-s * 1.1, s * 0.5] {
        let x = cx + dx;
        triangle(
            batch,
            atlas,
            [x - s * 0.6, cy - s * 1.3],
            [x + s * 0.9, cy],
            [x - s * 0.6, cy + s * 1.3],
        );
    }
}

/// The inventory: three dots, "...".
fn dots(batch: &mut HudBatch, atlas: &HudAtlas, r: Rect) {
    let (cx, cy) = ((r.x + r.w / 2.0).round(), (r.y + r.h / 2.0).round());
    for dx in [-6.0, -1.0, 4.0] {
        batch.fill(atlas, cx + dx, cy - 1.0, 2.0, 2.0, WHITE);
    }
}

/// Pause: three bars, ≡.
fn bars(batch: &mut HudBatch, atlas: &HudAtlas, r: Rect) {
    let (cx, cy) = ((r.x + r.w / 2.0).round(), (r.y + r.h / 2.0).round());
    for dy in [-5.0, -1.0, 3.0] {
        batch.fill(atlas, cx - 5.0, cy + dy, 10.0, 2.0, WHITE);
    }
}

/// Chat: a speech bubble.
fn bubble(batch: &mut HudBatch, atlas: &HudAtlas, r: Rect) {
    let (cx, cy) = ((r.x + r.w / 2.0).round(), (r.y + r.h / 2.0).round());
    batch.fill(atlas, cx - 6.0, cy - 5.0, 12.0, 8.0, WHITE);
    triangle(
        batch,
        atlas,
        [cx - 4.0, cy + 3.0],
        [cx, cy + 3.0],
        [cx - 4.0, cy + 6.0],
    );
    let dark = rgba(40, 40, 40, 200);
    for dx in [-4.0, -1.0, 2.0] {
        batch.fill(atlas, cx + dx, cy - 2.0, 2.0, 2.0, dark);
    }
}

/// Perspective: an eye.
fn eye(batch: &mut HudBatch, atlas: &HudAtlas, r: Rect) {
    let (cx, cy) = ((r.x + r.w / 2.0).round(), (r.y + r.h / 2.0).round());
    batch.fill(atlas, cx - 6.0, cy - 1.0, 12.0, 2.0, WHITE);
    batch.fill(atlas, cx - 4.0, cy - 3.0, 8.0, 6.0, WHITE);
    batch.fill(atlas, cx - 2.0, cy - 4.0, 4.0, 8.0, WHITE);
    batch.fill(atlas, cx - 1.0, cy - 1.0, 2.0, 2.0, rgba(40, 40, 40, 220));
}

/// Hide the keyboard: keys over an arrow pointing down.
fn keyboard_down(batch: &mut HudBatch, atlas: &HudAtlas, r: Rect) {
    let (cx, cy) = ((r.x + r.w / 2.0).round(), (r.y + r.h / 2.0).round());
    for row in 0..2 {
        for col in 0..4 {
            let (x, y) = (cx - 7.0 + col as f32 * 4.0, cy - 6.0 + row as f32 * 3.0);
            batch.fill(atlas, x, y, 2.0, 2.0, WHITE);
        }
    }
    triangle(
        batch,
        atlas,
        [cx - 4.0, cy + 1.0],
        [cx + 4.0, cy + 1.0],
        [cx, cy + 6.0],
    );
}

/// Close: an X.
fn cross(batch: &mut HudBatch, atlas: &HudAtlas, r: Rect) {
    let (cx, cy) = (r.x + r.w / 2.0, r.y + r.h / 2.0);
    let (s, t) = (r.w * 0.25, 1.1);
    for d in [1.0f32, -1.0] {
        batch.fill_quad(
            atlas,
            [
                [cx - s - t, cy - d * (s - t)],
                [cx - s + t, cy - d * (s + t)],
                [cx + s + t, cy + d * (s - t)],
                [cx + s - t, cy + d * (s + t)],
            ],
            WHITE,
        );
    }
}
