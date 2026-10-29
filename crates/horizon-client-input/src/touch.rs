//! Touch controls (Horizon's own, for iOS), after Bedrock Edition's with split controls: a
//! floating joystick on the left moves, a drag anywhere else turns the view, a tap there uses or
//! places (or hits the mob under the crosshair) and a hold breaks. Buttons jump, sneak, open the
//! inventory, change the perspective and pause, and Use (shown while the crosshair is on an
//! entity, Bedrock's Interact) trades, feeds or rides; the hotbar's slots are tapped, and held to
//! drop.
//!
//! The controls are a virtual controller: each frame gives a [`PadState`] the game handles as a
//! pad's (analog movement, toggle sneak, held and tapped triggers), plus the view's turn and the
//! hotbar slot chosen. Positions are GUI pixels, as the HUD draws them.
use crate::gamepad::{Button, PadState};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// A held finger turns into breaking after this long without moving far.
pub const HOLD_TO_BREAK: Duration = Duration::from_millis(300);
/// A finger lifted sooner than this, without moving far, is a tap.
pub const TAP: Duration = Duration::from_millis(250);
/// A hotbar slot held this long drops its item.
pub const HOLD_TO_DROP: Duration = Duration::from_millis(600);
/// How far (GUI pixels) a finger may move and still tap or hold.
pub const SLOP: f32 = 6.0;
/// The joystick's reach from where the thumb landed (GUI pixels).
pub const STICK_RADIUS: f32 = 26.0;
/// Past this tilt, mostly forward, the joystick sprints.
const SPRINT_TILT: f32 = 0.9;

/// A rectangle in GUI pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }
    pub fn contains(&self, [x, y]: [f32; 2]) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }
}

/// The on-screen buttons.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TouchButton {
    Jump,
    Sneak,
    Sprint,
    Inventory,
    Chat,
    Perspective,
    Pause,
    Use,
}

impl TouchButton {
    pub const ALL: [TouchButton; 8] = [
        TouchButton::Jump,
        TouchButton::Sneak,
        TouchButton::Sprint,
        TouchButton::Inventory,
        TouchButton::Chat,
        TouchButton::Perspective,
        TouchButton::Pause,
        TouchButton::Use,
    ];
    /// What it presses on the controller (docs/controllers.md's Bedrock layout).
    pub fn press(self, pad: &mut PadState) {
        match self {
            TouchButton::Jump => pad.set(Button::South, true),
            TouchButton::Sneak => pad.set(Button::RightStick, true),
            TouchButton::Sprint => pad.set(Button::LeftStick, true),
            TouchButton::Chat => pad.set(Button::DPadRight, true),
            TouchButton::Inventory => pad.set(Button::North, true),
            TouchButton::Perspective => pad.set(Button::DPadUp, true),
            TouchButton::Pause => pad.set(Button::Start, true),
            TouchButton::Use => pad.triggers[0] = 1.0,
        }
    }
}

/// Where the controls are on a GUI `width` x `height` screen.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TouchLayout {
    pub width: f32,
    pub height: f32,
    /// Kept clear at the left and right edges: a phone's notch or rounded corners.
    pub inset: f32,
}

impl TouchLayout {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            width,
            height,
            inset: if width >= 480.0 { 36.0 } else { 8.0 },
        }
    }
    /// Hotbar slot `n` (0..9), as vanilla draws the bar: 182x22 centred at the bottom.
    pub fn hotbar_slot(&self, n: usize) -> Rect {
        let left = (self.width / 2.0).floor() - 91.0;
        Rect::new(left + n as f32 * 20.0 + 1.0, self.height - 22.0, 20.0, 22.0)
    }
    /// Bedrock's layout: jump on the right with sneak under it and sprint to its left, the
    /// small chat, perspective and pause buttons along the top, "..." at the hotbar's end.
    pub fn button(&self, button: TouchButton) -> Rect {
        let (w, h, m) = (self.width, self.height, self.inset);
        let big = 30.0;
        let small = 18.0;
        let jump = Rect::new(w - m - big, (h * 0.55).floor(), big, big);
        let top = |n: f32| Rect::new(w - m - small - n * (small + 5.0), 4.0, small, small);
        match button {
            TouchButton::Jump => jump,
            TouchButton::Sneak => Rect::new(jump.x, jump.y + big + 5.0, big, big),
            TouchButton::Sprint => Rect::new(jump.x - big - 5.0, jump.y, big, big),
            TouchButton::Inventory => {
                let slot = self.hotbar_slot(8);
                Rect::new(slot.x + slot.w + 3.0, h - 22.0, 22.0, 22.0)
            }
            TouchButton::Pause => top(0.0),
            TouchButton::Chat => top(1.0),
            TouchButton::Perspective => top(2.0),
            // Below and right of the crosshair, under the right thumb.
            TouchButton::Use => Rect::new((w / 2.0).floor() + 24.0, (h / 2.0).floor() + 14.0, 40.0, 18.0),
        }
    }
    /// Where a thumb starts the joystick: the left part of the screen, below the top.
    pub fn joystick_zone(&self) -> Rect {
        Rect::new(0.0, 28.0, self.width * 0.4, self.height - 28.0)
    }
    /// The joystick's centre while no thumb is on it.
    pub fn joystick_rest(&self) -> [f32; 2] {
        [self.inset + 40.0, self.height - 56.0]
    }
    /// The close button over menus (Escape), top right.
    pub fn close(&self) -> Rect {
        Rect::new(self.width - self.inset - 20.0, 4.0, 20.0, 20.0)
    }
    /// The button that hides the system keyboard, left of the close button: the iPhone's
    /// keyboard has no key of its own for it.
    pub fn keyboard_hide(&self) -> Rect {
        let close = self.close();
        Rect::new(close.x - close.w - 5.0, close.y, 20.0, 20.0)
    }
    /// The Shift toggle under it in inventories: taps quick-move while it is on.
    pub fn shift(&self) -> Rect {
        let close = self.close();
        Rect::new(close.x, close.y + close.h + 5.0, 20.0, 20.0)
    }
}

/// What a finger does, from where it went down.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Role {
    Hotbar {
        slot: usize,
        since: Instant,
        dropped: bool,
    },
    /// A button, and whether a frame has seen it held.
    Button(TouchButton, bool),
    Joystick {
        origin: [f32; 2],
        at: [f32; 2],
    },
    Look {
        since: Instant,
        start: [f32; 2],
        last: [f32; 2],
        breaking: bool,
    },
    /// Down while a screen opened: ignored until lifted.
    Ignored,
}

/// One frame of the controls.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TouchFrame {
    /// Movement, the buttons held, and breaking (the right trigger pulled).
    pub pad: PadState,
    /// The view's turn since the last frame, in GUI pixels.
    pub look: [f32; 2],
    /// A tap on the view: use the held item or place, or attack the mob under the crosshair.
    pub tap: bool,
    /// The hotbar slot tapped.
    pub hotbar: Option<usize>,
    /// A hotbar slot was held: drop the selected item.
    pub drop: bool,
}

/// The fingers on the screen and what each does.
#[derive(Debug)]
pub struct TouchControls {
    pub layout: TouchLayout,
    /// The crosshair is on an entity: the Use button shows.
    pub use_shown: bool,
    fingers: HashMap<u64, Role>,
    look: [f32; 2],
    tap: bool,
    hotbar: Option<usize>,
    drop: bool,
    /// Buttons lifted before any frame saw them: iOS often delivers a touch near the screen's
    /// edge only when it ends, so its start and end come between two frames.
    quick: Vec<TouchButton>,
}

impl Default for TouchControls {
    fn default() -> Self {
        Self {
            layout: TouchLayout::new(0.0, 0.0),
            use_shown: false,
            fingers: HashMap::new(),
            look: [0.0; 2],
            tap: false,
            hotbar: None,
            drop: false,
            quick: Vec::new(),
        }
    }
}

fn distance(a: [f32; 2], b: [f32; 2]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

impl TouchControls {
    /// The finger is one of the controls' (down since the game was playing).
    pub fn owns(&self, id: u64) -> bool {
        self.fingers.contains_key(&id)
    }

    pub fn start(&mut self, id: u64, at: [f32; 2], now: Instant) {
        let layout = self.layout;
        let role = if let Some(slot) = (0..9).find(|&n| layout.hotbar_slot(n).contains(at)) {
            self.hotbar = Some(slot);
            Role::Hotbar {
                slot,
                since: now,
                dropped: false,
            }
        } else if let Some(button) = TouchButton::ALL
            .into_iter()
            .find(|&b| self.shown(b) && layout.button(b).contains(at))
        {
            Role::Button(button, false)
        } else if layout.joystick_zone().contains(at) && !self.has_joystick() {
            Role::Joystick { origin: at, at }
        } else {
            Role::Look {
                since: now,
                start: at,
                last: at,
                breaking: false,
            }
        };
        self.fingers.insert(id, role);
    }

    pub fn moved(&mut self, id: u64, to: [f32; 2]) {
        match self.fingers.get_mut(&id) {
            Some(Role::Joystick { at, .. }) => *at = to,
            Some(Role::Look { last, .. }) => {
                self.look[0] += to[0] - last[0];
                self.look[1] += to[1] - last[1];
                *last = to;
            }
            _ => {}
        }
    }

    pub fn end(&mut self, id: u64, at: [f32; 2], now: Instant) {
        self.moved(id, at);
        match self.fingers.remove(&id) {
            Some(Role::Look {
                since,
                start,
                breaking: false,
                ..
            }) if now.duration_since(since) < TAP && distance(start, at) <= SLOP => {
                self.tap = true;
            }
            Some(Role::Button(button, false)) => self.quick.push(button),
            _ => {}
        }
    }

    /// A screen opened: every finger down stops acting until it is lifted, and nothing pending
    /// carries over.
    pub fn release_all(&mut self) {
        for role in self.fingers.values_mut() {
            *role = Role::Ignored;
        }
        self.look = [0.0; 2];
        self.tap = false;
        self.hotbar = None;
        self.drop = false;
        self.quick.clear();
    }

    /// The button is on screen.
    pub fn shown(&self, button: TouchButton) -> bool {
        button != TouchButton::Use || self.use_shown
    }

    fn has_joystick(&self) -> bool {
        self.fingers
            .values()
            .any(|r| matches!(r, Role::Joystick { .. }))
    }

    /// The joystick's tilt, right and up positive, at most 1 long.
    pub fn stick(&self) -> Option<([f32; 2], [f32; 2])> {
        self.fingers.values().find_map(|r| match *r {
            Role::Joystick { origin, at } => {
                let mut v = [
                    (at[0] - origin[0]) / STICK_RADIUS,
                    (origin[1] - at[1]) / STICK_RADIUS,
                ];
                let len = (v[0] * v[0] + v[1] * v[1]).sqrt();
                if len > 1.0 {
                    v = [v[0] / len, v[1] / len];
                }
                Some((origin, v))
            }
            _ => None,
        })
    }

    /// A button is held (drawn highlighted).
    pub fn held(&self, button: TouchButton) -> bool {
        self.fingers
            .values()
            .any(|r| matches!(r, Role::Button(b, _) if *b == button))
    }

    /// This frame's controls; the turn, taps and slot changes since the last are taken.
    pub fn frame(&mut self, now: Instant) -> TouchFrame {
        let mut pad = PadState::default();
        for button in std::mem::take(&mut self.quick) {
            button.press(&mut pad);
        }
        for role in self.fingers.values_mut() {
            match role {
                Role::Button(button, seen) => {
                    button.press(&mut pad);
                    *seen = true;
                }
                Role::Look {
                    since,
                    start,
                    last,
                    breaking,
                } => {
                    if !*breaking
                        && now.duration_since(*since) >= HOLD_TO_BREAK
                        && distance(*start, *last) <= SLOP
                    {
                        *breaking = true;
                    }
                    if *breaking {
                        pad.triggers[1] = 1.0;
                    }
                }
                Role::Hotbar {
                    since, dropped, ..
                } => {
                    if !*dropped && now.duration_since(*since) >= HOLD_TO_DROP {
                        *dropped = true;
                        self.drop = true;
                    }
                }
                Role::Joystick { .. } | Role::Ignored => {}
            }
        }
        if let Some((_, v)) = self.stick() {
            pad.left = v;
            let tilt = (v[0] * v[0] + v[1] * v[1]).sqrt();
            if tilt >= SPRINT_TILT && v[1] > v[0].abs() {
                pad.set(Button::LeftStick, true);
            }
        }
        TouchFrame {
            pad,
            look: std::mem::take(&mut self.look),
            tap: std::mem::take(&mut self.tap),
            hotbar: self.hotbar.take(),
            drop: std::mem::take(&mut self.drop),
        }
    }

    /// The fingers on the hotbar, for drawing; and whether the slot `n` is held.
    pub fn hotbar_held(&self, n: usize) -> bool {
        self.fingers
            .values()
            .any(|r| matches!(r, Role::Hotbar { slot, .. } if *slot == n))
    }
}

/// A menu finger held this long without moving presses where it is: the right button over an
/// inventory's slots (split a stack, place one), else the left (drag a slider or scrollbar).
pub const MENU_HOLD: Duration = Duration::from_millis(350);

/// What a finger in a menu does as the mouse, in GUI pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MenuAction {
    Move([f32; 2]),
    Press { right: bool },
    Release { right: bool },
    /// The list under the finger follows it: positive when the finger moved down.
    Scroll(f32),
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum MenuMode {
    /// Down, not yet a tap, a scroll or a drag.
    Undecided,
    Scroll,
    Drag { right: bool },
}

#[derive(Clone, Copy, Debug)]
struct MenuFinger {
    id: u64,
    start: [f32; 2],
    last: [f32; 2],
    since: Instant,
    mode: MenuMode,
}

/// The first finger in a menu, as the mouse. Buttons in menus act when pressed, so the press
/// waits until the finger shows what it does: lifted, it was a tap (a click); moved up or down,
/// it scrolls the list (no click); moved sideways, or over an inventory's slots, it drags with
/// the button down from where it started; held still, it presses there (see [`MENU_HOLD`]).
#[derive(Debug, Default)]
pub struct MenuTouch {
    finger: Option<MenuFinger>,
}

impl MenuTouch {
    pub fn owns(&self, id: u64) -> bool {
        self.finger.is_some_and(|f| f.id == id)
    }

    pub fn start(&mut self, id: u64, at: [f32; 2], now: Instant) -> Vec<MenuAction> {
        if self.finger.is_some() {
            return Vec::new();
        }
        self.finger = Some(MenuFinger {
            id,
            start: at,
            last: at,
            since: now,
            mode: MenuMode::Undecided,
        });
        vec![MenuAction::Move(at)]
    }

    /// `slots`: the menu is an inventory, where a drag moves items instead of scrolling.
    pub fn moved(&mut self, id: u64, at: [f32; 2], slots: bool) -> Vec<MenuAction> {
        let Some(f) = self.finger.as_mut().filter(|f| f.id == id) else {
            return Vec::new();
        };
        let actions = match f.mode {
            MenuMode::Undecided if distance(f.start, at) <= SLOP => Vec::new(),
            MenuMode::Undecided => {
                let (dx, dy) = (at[0] - f.start[0], at[1] - f.start[1]);
                if !slots && dy.abs() > dx.abs() {
                    f.mode = MenuMode::Scroll;
                    vec![MenuAction::Scroll(dy)]
                } else {
                    f.mode = MenuMode::Drag { right: false };
                    vec![
                        MenuAction::Move(f.start),
                        MenuAction::Press { right: false },
                        MenuAction::Move(at),
                    ]
                }
            }
            MenuMode::Scroll => vec![MenuAction::Scroll(at[1] - f.last[1])],
            MenuMode::Drag { .. } => vec![MenuAction::Move(at)],
        };
        f.last = at;
        actions
    }

    pub fn end(&mut self, id: u64, at: [f32; 2]) -> Vec<MenuAction> {
        if !self.owns(id) {
            return Vec::new();
        }
        let Some(f) = self.finger.take() else {
            return Vec::new();
        };
        match f.mode {
            MenuMode::Undecided => vec![
                MenuAction::Move(f.start),
                MenuAction::Press { right: false },
                MenuAction::Release { right: false },
            ],
            MenuMode::Scroll => Vec::new(),
            MenuMode::Drag { right } => vec![MenuAction::Move(at), MenuAction::Release { right }],
        }
    }

    /// Once per frame: a finger held still long enough presses.
    pub fn poll(&mut self, now: Instant, slots: bool) -> Vec<MenuAction> {
        match self.finger.as_mut() {
            Some(f)
                if f.mode == MenuMode::Undecided && now.duration_since(f.since) >= MENU_HOLD =>
            {
                f.mode = MenuMode::Drag { right: slots };
                vec![MenuAction::Move(f.last), MenuAction::Press { right: slots }]
            }
            _ => Vec::new(),
        }
    }

    /// The menu closed or changed: forget the finger (its lift is then ignored).
    pub fn cancel(&mut self) {
        self.finger = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn controls() -> TouchControls {
        TouchControls {
            // An iPhone 12 at auto GUI scale 4.
            layout: TouchLayout::new(633.0, 293.0),
            ..Default::default()
        }
    }

    fn centre(r: Rect) -> [f32; 2] {
        [r.x + r.w / 2.0, r.y + r.h / 2.0]
    }

    #[test]
    fn menu_tap_clicks_on_lift() {
        let t0 = Instant::now();
        let mut m = MenuTouch::default();
        assert_eq!(m.start(1, [100.0, 50.0], t0), [MenuAction::Move([100.0, 50.0])]);
        assert!(m.moved(1, [102.0, 51.0], false).is_empty(), "within the slop");
        assert!(m.poll(t0 + Duration::from_millis(100), false).is_empty());
        assert_eq!(
            m.end(1, [102.0, 51.0]),
            [
                MenuAction::Move([100.0, 50.0]),
                MenuAction::Press { right: false },
                MenuAction::Release { right: false },
            ]
        );
        assert!(!m.owns(1));
    }

    #[test]
    fn menu_vertical_drag_scrolls_without_clicking() {
        let t0 = Instant::now();
        let mut m = MenuTouch::default();
        m.start(1, [100.0, 50.0], t0);
        assert_eq!(m.moved(1, [101.0, 70.0], false), [MenuAction::Scroll(20.0)]);
        assert_eq!(m.moved(1, [101.0, 60.0], false), [MenuAction::Scroll(-10.0)]);
        assert!(m.poll(t0 + MENU_HOLD * 2, false).is_empty(), "scrolling never presses");
        assert!(m.end(1, [101.0, 60.0]).is_empty());
    }

    #[test]
    fn menu_sideways_drag_presses_where_it_started() {
        let t0 = Instant::now();
        let mut m = MenuTouch::default();
        m.start(1, [100.0, 50.0], t0);
        assert_eq!(
            m.moved(1, [120.0, 52.0], false),
            [
                MenuAction::Move([100.0, 50.0]),
                MenuAction::Press { right: false },
                MenuAction::Move([120.0, 52.0]),
            ]
        );
        assert_eq!(
            m.end(1, [130.0, 52.0]),
            [MenuAction::Move([130.0, 52.0]), MenuAction::Release { right: false }]
        );
    }

    #[test]
    fn inventory_drags_and_long_presses() {
        let t0 = Instant::now();
        let mut m = MenuTouch::default();
        // Up and down over slots drags an item instead of scrolling.
        m.start(1, [100.0, 50.0], t0);
        assert_eq!(m.moved(1, [100.0, 80.0], true)[1], MenuAction::Press { right: false });
        m.end(1, [100.0, 80.0]);
        // Held still: the right button.
        m.start(2, [100.0, 50.0], t0);
        assert!(m.poll(t0 + MENU_HOLD / 2, true).is_empty());
        assert_eq!(
            m.poll(t0 + MENU_HOLD, true),
            [MenuAction::Move([100.0, 50.0]), MenuAction::Press { right: true }]
        );
        assert!(m.poll(t0 + MENU_HOLD * 2, true).is_empty(), "once");
        assert_eq!(m.end(2, [100.0, 50.0])[1], MenuAction::Release { right: true });
        // Elsewhere a hold presses the left button (a slider).
        m.start(3, [100.0, 50.0], t0);
        assert_eq!(m.poll(t0 + MENU_HOLD, false)[1], MenuAction::Press { right: false });
    }

    #[test]
    fn menu_second_finger_is_ignored() {
        let t0 = Instant::now();
        let mut m = MenuTouch::default();
        m.start(1, [100.0, 50.0], t0);
        assert!(m.start(2, [200.0, 50.0], t0).is_empty());
        assert!(m.end(2, [200.0, 50.0]).is_empty());
        assert!(m.owns(1));
    }

    #[test]
    fn layout_fits_and_does_not_overlap() {
        let l = TouchLayout::new(633.0, 293.0);
        let mut rects: Vec<Rect> = TouchButton::ALL.iter().map(|&b| l.button(b)).collect();
        rects.extend((0..9).map(|n| l.hotbar_slot(n)));
        for (i, a) in rects.iter().enumerate() {
            assert!(a.x >= 0.0 && a.y >= 0.0 && a.x + a.w <= 633.0 && a.y + a.h <= 293.0);
            for b in &rects[i + 1..] {
                let apart = a.x + a.w <= b.x
                    || b.x + b.w <= a.x
                    || a.y + a.h <= b.y
                    || b.y + b.h <= a.y;
                assert!(apart, "{a:?} overlaps {b:?}");
            }
        }
    }

    #[test]
    fn buttons_hold_their_pad_buttons() {
        let now = Instant::now();
        let mut c = controls();
        c.start(1, centre(c.layout.button(TouchButton::Jump)), now);
        c.start(2, centre(c.layout.button(TouchButton::Sneak)), now);
        let f = c.frame(now);
        assert!(f.pad.held(Button::South));
        assert!(f.pad.held(Button::RightStick));
        c.end(1, centre(c.layout.button(TouchButton::Jump)), now);
        assert!(!c.frame(now).pad.held(Button::South));
    }

    #[test]
    fn a_tap_between_frames_still_presses_once() {
        let now = Instant::now();
        let mut c = controls();
        let jump = centre(c.layout.button(TouchButton::Jump));
        c.start(1, jump, now);
        c.end(1, jump, now);
        assert!(c.frame(now).pad.held(Button::South));
        assert!(!c.frame(now).pad.held(Button::South), "once");
        // Seen held by a frame: released when lifted.
        c.start(2, jump, now);
        assert!(c.frame(now).pad.held(Button::South));
        c.end(2, jump, now);
        assert!(!c.frame(now).pad.held(Button::South));
    }

    #[test]
    fn use_button_only_while_shown() {
        let now = Instant::now();
        let mut c = controls();
        let at = centre(c.layout.button(TouchButton::Use));
        c.start(1, at, now);
        assert_eq!(c.frame(now).pad.triggers[0], 0.0, "hidden: the view takes it");
        c.end(1, at, now);
        assert!(c.frame(now).tap);
        c.use_shown = true;
        c.start(2, at, now);
        let f = c.frame(now);
        assert_eq!(f.pad.triggers[0], 1.0);
        assert!(!f.tap);
    }

    #[test]
    fn joystick_floats_clamps_and_sprints() {
        let now = Instant::now();
        let mut c = controls();
        c.start(1, [100.0, 200.0], now);
        c.moved(1, [100.0 + STICK_RADIUS / 2.0, 200.0]);
        let f = c.frame(now);
        assert!((f.pad.left[0] - 0.5).abs() < 1e-5 && f.pad.left[1].abs() < 1e-5);
        assert!(!f.pad.held(Button::LeftStick));
        // Far forward: clamped to 1, and sprinting.
        c.moved(1, [100.0, 200.0 - 3.0 * STICK_RADIUS]);
        let f = c.frame(now);
        assert!((f.pad.left[1] - 1.0).abs() < 1e-5);
        assert!(f.pad.held(Button::LeftStick));
        // Fully sideways doesn't sprint.
        c.moved(1, [100.0 + 3.0 * STICK_RADIUS, 200.0]);
        assert!(!c.frame(now).pad.held(Button::LeftStick));
        // A second thumb on the left looks instead of taking the joystick over.
        c.start(2, [50.0, 150.0], now);
        c.moved(2, [60.0, 150.0]);
        assert_eq!(c.frame(now).look, [10.0, 0.0]);
    }

    #[test]
    fn tap_hold_and_drag_on_the_view() {
        let t0 = Instant::now();
        let ms = |n| t0 + Duration::from_millis(n);
        let at = [450.0, 120.0];
        let mut c = controls();
        // A tap.
        c.start(1, at, t0);
        c.end(1, at, ms(100));
        let f = c.frame(ms(100));
        assert!(f.tap && f.pad.triggers[1] == 0.0);
        assert!(!c.frame(ms(120)).tap, "a tap is taken once");
        // A hold breaks until lifted, and is no tap.
        c.start(2, at, ms(200));
        assert!(c.frame(ms(300)).pad.triggers[1] == 0.0);
        assert!(c.frame(ms(520)).pad.triggers[1] > 0.0);
        c.moved(2, [480.0, 120.0]);
        let f = c.frame(ms(600));
        assert!(f.pad.triggers[1] > 0.0, "breaking survives moving");
        assert_eq!(f.look, [30.0, 0.0]);
        c.end(2, [480.0, 120.0], ms(700));
        let f = c.frame(ms(700));
        assert!(!f.tap && f.pad.triggers[1] == 0.0);
        // A drag turns and is neither.
        c.start(3, at, ms(800));
        c.moved(3, [460.0, 100.0]);
        c.end(3, [470.0, 90.0], ms(850));
        let f = c.frame(ms(1200));
        assert_eq!(f.look, [20.0, -30.0]);
        assert!(!f.tap && f.pad.triggers[1] == 0.0);
    }

    #[test]
    fn hotbar_selects_and_drops_when_held() {
        let t0 = Instant::now();
        let mut c = controls();
        c.start(1, centre(c.layout.hotbar_slot(4)), t0);
        let f = c.frame(t0);
        assert_eq!(f.hotbar, Some(4));
        assert!(!f.drop);
        assert_eq!(c.frame(t0).hotbar, None, "a slot is chosen once");
        let f = c.frame(t0 + HOLD_TO_DROP);
        assert!(f.drop);
        assert!(!c.frame(t0 + HOLD_TO_DROP * 2).drop, "one drop per hold");
    }

    #[test]
    fn release_all_ignores_fingers_until_lifted() {
        let t0 = Instant::now();
        let mut c = controls();
        c.start(1, centre(c.layout.button(TouchButton::Inventory)), t0);
        c.start(2, [100.0, 200.0], t0);
        c.moved(2, [120.0, 200.0]);
        c.release_all();
        assert_eq!(c.frame(t0), TouchFrame::default());
        assert!(c.owns(1));
        c.end(1, [0.0, 0.0], t0);
        assert!(!c.owns(1));
    }
}
