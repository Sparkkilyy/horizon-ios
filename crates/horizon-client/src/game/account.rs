//! The Microsoft account on iOS, where there is no launcher to sign in (docs/microsoft-account.md):
//! Options › Horizon Settings › Microsoft Account opens this screen. Sign-in is the launcher's
//! device-code flow (horizon-account): the player enters a code at microsoft.com/link, the game
//! polls in the background and keeps the account in its sandboxed Library, renewed at each start.
//! The signed-in account is the session online-mode servers are joined with (`auth::current`).
use super::Game;
use horizon_account::{NOT_CONFIGURED, Saved, SignIn, SignInState};
use horizon_client::{
    auth::{self, AuthSession},
    hud::{HudBatch, rgba},
    text::Text,
    ui,
};
use std::path::PathBuf;

/// The open account screen and the sign-in under way, if any.
#[derive(Default)]
pub(super) struct AccountScreen {
    sign_in: Option<SignIn>,
    /// The last sign-in's error.
    error: Option<String>,
}

#[derive(Clone, Copy, PartialEq)]
enum Action {
    SignIn,
    SignOut,
    OpenLink,
    CopyCode,
    Cancel,
    Done,
}

/// Where the account is kept: the app's Library (not Documents, which the Files app shows).
fn saved_path() -> PathBuf {
    horizon_client::logging::data_dir()
        .join("horizon")
        .join("account.json")
}

fn use_account(saved: &Saved) {
    let uuid = auth::parse_uuid(&saved.account.uuid).unwrap_or_default();
    let mut session = AuthSession::new(&saved.account.name, uuid, saved.secrets.minecraft_token());
    session.xuid = saved.secrets.xuid().map(str::to_owned);
    auth::set_current(session);
}

/// At start: the saved account, renewed in the background (a network round trip), becomes the
/// session; a sign-in that can't be renewed any more is forgotten.
pub(super) fn restore_in_background() {
    let path = saved_path();
    let Some(saved) = Saved::load(&path) else {
        return;
    };
    // Usable at once while its token lasts; the renewal replaces it.
    use_account(&saved);
    std::thread::spawn(move || match saved.renew() {
        Ok(saved) => {
            use_account(&saved);
            if let Err(e) = saved.save(&path) {
                log::warn!("Horizon: could not save the Microsoft account: {e}");
            }
        }
        Err((e, still)) => {
            log::warn!("Horizon: Microsoft account not renewed: {e}");
            match still {
                Some(saved) => {
                    let _ = saved.save(&path);
                }
                None => {
                    auth::clear_current();
                    Saved::forget(&path);
                }
            }
        }
    });
}

impl Game<'_> {
    pub(super) fn open_account(&mut self) {
        self.pad.account = Some(AccountScreen::default());
    }

    pub(super) fn account_open(&self) -> bool {
        self.pad.account.is_some()
    }

    /// Once per frame: a finished sign-in becomes the account.
    fn account_poll(&mut self) {
        let Some(screen) = self.pad.account.as_mut() else {
            return;
        };
        let Some(SignInState::Done(result)) = screen.sign_in.as_ref().map(SignIn::state) else {
            return;
        };
        screen.sign_in = None;
        match result {
            Ok(saved) => {
                use_account(&saved);
                if let Err(e) = saved.save(&saved_path()) {
                    log::warn!("Horizon: could not save the Microsoft account: {e}");
                }
                screen.error = None;
            }
            Err(e) => screen.error = Some(e),
        }
    }

    /// The lines of text and the buttons, for drawing and for clicks.
    fn account_view(&self) -> (Vec<(String, u32)>, Vec<(Action, &'static str)>) {
        let white = rgba(255, 255, 255, 255);
        let grey = rgba(170, 170, 170, 255);
        let yellow = rgba(255, 255, 85, 255);
        let red = rgba(255, 85, 85, 255);
        let Some(screen) = &self.pad.account else {
            return (Vec::new(), Vec::new());
        };
        if horizon_account::client_id().is_none() {
            return (
                vec![(NOT_CONFIGURED.into(), red)],
                vec![(Action::Done, "Done")],
            );
        }
        match screen.sign_in.as_ref().map(SignIn::state) {
            Some(SignInState::Starting) => (
                vec![("Asking Microsoft for a sign-in code...".into(), grey)],
                vec![(Action::Cancel, "Cancel")],
            ),
            Some(SignInState::Code(code)) => (
                vec![
                    ("Open microsoft.com/link and enter this code:".into(), white),
                    (code.user_code.clone(), yellow),
                    ("Waiting for you to finish signing in...".into(), grey),
                ],
                vec![
                    (Action::OpenLink, "Open microsoft.com/link"),
                    (Action::CopyCode, "Copy Code"),
                    (Action::Cancel, "Cancel"),
                ],
            ),
            Some(SignInState::Done(_)) => (Vec::new(), Vec::new()),
            None => {
                let mut lines = Vec::new();
                if let Some(e) = &screen.error {
                    lines.push((e.clone(), red));
                }
                match auth::current() {
                    Some(session) => {
                        lines.push((format!("Signed in as {}", session.name), white));
                        lines.push(("Online servers use this account.".into(), grey));
                        (
                            lines,
                            vec![(Action::SignOut, "Sign Out"), (Action::Done, "Done")],
                        )
                    }
                    None => {
                        lines.push((
                            "Sign in to join online servers with your own name and skin.".into(),
                            white,
                        ));
                        lines.push((
                            "The account must own Minecraft: Java Edition.".into(),
                            grey,
                        ));
                        (
                            lines,
                            vec![(Action::SignIn, "Sign In"), (Action::Done, "Done")],
                        )
                    }
                }
            }
        }
    }

    /// Each button's rectangle: a column under the text, centred.
    fn account_buttons(
        &self,
        size: (f32, f32),
        lines: usize,
        buttons: &[(Action, &'static str)],
    ) -> Vec<(Action, &'static str, [f32; 4])> {
        let (w, h) = size;
        let top = (h / 2.0 - 10.0 - lines as f32 * 6.0).max(40.0) + lines as f32 * 12.0 + 12.0;
        buttons
            .iter()
            .enumerate()
            .map(|(i, &(action, label))| {
                (action, label, [w / 2.0 - 100.0, top + i as f32 * 24.0, 200.0, 20.0])
            })
            .collect()
    }

    pub(super) fn draw_account(&mut self, batch: &mut HudBatch) {
        if !self.account_open() {
            return;
        }
        self.account_poll();
        let atlas = &self.gui.atlas;
        let (w, h) = (batch.width, batch.height);
        ui::menu_background(batch, atlas, 0.0, 0.0, w, h, true);
        let title = "Microsoft Account";
        let tw = HudBatch::text_width(atlas, title);
        let (lines, buttons) = self.account_view();
        let top = (h / 2.0 - 10.0 - lines.len() as f32 * 6.0).max(40.0);
        batch.text(atlas, title, (w - tw) / 2.0, top - 24.0, rgba(255, 255, 255, 255), true);
        for (i, (line, color)) in lines.iter().enumerate() {
            let lw = HudBatch::text_width(atlas, line);
            batch.text(atlas, line, ((w - lw) / 2.0).max(4.0), top + i as f32 * 12.0, *color, true);
        }
        for (_, label, [x, y, bw, bh]) in self.account_buttons((w, h), lines.len(), &buttons) {
            let hover = ui::hovered(self.cursor, x, y, bw, bh);
            ui::button(batch, atlas, x, y, bw, bh, &Text::literal(label), true, hover);
        }
    }

    /// A click on the account screen (it takes every click while open).
    pub(super) fn account_click(&mut self) {
        let (lines, buttons) = self.account_view();
        let size = self.gui_size();
        let Some(action) = self
            .account_buttons(size, lines.len(), &buttons)
            .into_iter()
            .find(|&(_, _, [x, y, w, h])| ui::hovered(self.cursor, x, y, w, h))
            .map(|(action, ..)| action)
        else {
            return;
        };
        let Some(screen) = self.pad.account.as_mut() else {
            return;
        };
        match action {
            Action::SignIn => {
                screen.error = None;
                screen.sign_in = Some(SignIn::start());
            }
            Action::SignOut => {
                auth::clear_current();
                Saved::forget(&saved_path());
            }
            Action::OpenLink | Action::CopyCode => {
                if let Some(SignInState::Code(code)) = screen.sign_in.as_ref().map(SignIn::state) {
                    if action == Action::OpenLink {
                        horizon_ios::open_url(&format!(
                            "https://www.microsoft.com/link?otc={}",
                            code.user_code
                        ));
                    } else {
                        horizon_ios::copy_text(&code.user_code);
                    }
                }
            }
            // Dropping the sign-in cancels it.
            Action::Cancel => screen.sign_in = None,
            Action::Done => self.pad.account = None,
        }
    }
}
