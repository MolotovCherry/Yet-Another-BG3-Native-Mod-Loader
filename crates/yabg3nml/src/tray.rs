use std::thread::{self, JoinHandle};

use tray_icon::{
    Icon, TrayIconBuilder,
    menu::{AboutMetadata, Menu, MenuEvent, MenuItem, PredefinedMenuItem},
};
use windows::Win32::{
    Foundation::{LPARAM, WPARAM},
    UI::WindowsAndMessaging::{GetClassNameW, PostMessageW, WM_CLOSE},
};

use crate::{
    RunType,
    stop_token::StopToken,
    wapi::{enum_windows::EnumWindowsRs, event_loop::EventLoop},
};

pub struct AppTray;

impl AppTray {
    pub fn run(
        watcher_token: StopToken,
        timeout_token: Option<StopToken>,
        kind: RunType,
    ) -> JoinHandle<()> {
        thread::spawn(move || {
            let icon = Icon::from_resource(1, None).unwrap();

            let tray_menu = Menu::new();

            let quit_i = MenuItem::new("Quit", true, None);

            let authors = env!("CARGO_PKG_AUTHORS")
                .split(':')
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>();

            let kind = match kind {
                RunType::Watcher => "Watcher".to_owned(),
                RunType::Injector => "Injector".to_owned(),
            };

            let title = format!("Yet Another BG3 Native Mod Loader - {kind}");

            tray_menu
                .append_items(&[
                    &PredefinedMenuItem::about(
                        None,
                        Some(AboutMetadata {
                            name: Some(title.clone()),
                            copyright: Some(format!("Copyright (c) {}", authors.join(", ")).to_owned()),
                            version: Some(env!("CARGO_PKG_VERSION").to_owned()),
                            authors: Some(authors),
                            license: Some(env!("CARGO_PKG_LICENSE").to_owned()),
                            website_label: Some("https://www.nexusmods.com/baldursgate3/mods/3052".to_owned()),
                            website: Some(env!("CARGO_PKG_HOMEPAGE").to_owned()),
                            comments: Some(r#"THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE."#.to_owned()),
                            ..Default::default()
                        }),
                    ),
                    &PredefinedMenuItem::separator(),
                    &quit_i,
                ])
                .unwrap();

            let mut tray_icon = Some(
                TrayIconBuilder::new()
                    .with_tooltip(title)
                    .with_menu(Box::new(tray_menu))
                    .with_icon(icon)
                    .build()
                    .unwrap(),
            );

            EventLoop::new().run(move |event_loop, _| {
                if let Ok(event) = MenuEvent::receiver().try_recv() {
                    if event.id == quit_i.id() {
                        if let Some(token) = timeout_token.as_ref() {
                            token.stop();
                        }

                        watcher_token.stop();
                        event_loop.exit();

                        tray_icon.take();

                        // this will close dialog popup window in injector mode so it doesn't hang process watcher
                        // when we try to quit. Would work for anything else hanging a thread too

                        // "#32770" - this lets us avoid string conversions
                        let class = [35u16, 51, 50, 55, 55, 48];

                        let mut buf = [0u16; 7];
                        EnumWindowsRs(|hwnd| {
                            let len = unsafe { GetClassNameW(hwnd, &mut buf) };
                            if len == 0 {
                                // fn call failed, but it doesn't matter
                                return Ok(());
                            }

                            let buf = &buf[..len as usize];

                            // looking for any open dialog box
                            if buf == class {
                                // close the window
                                _ = unsafe {
                                    PostMessageW(hwnd.into(), WM_CLOSE, WPARAM(0), LPARAM(0))
                                };
                            }

                            Ok(())
                        });
                    }
                }
            });
        })
    }
}
