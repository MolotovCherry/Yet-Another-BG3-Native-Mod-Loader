use std::{
    process,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
};

use tray_icon::{
    menu::{AboutMetadata, Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    Icon, TrayIconBuilder,
};

use crate::{event_loop::EventLoop, stop_token::StopToken};

pub struct AppTray;

impl AppTray {
    pub fn run(
        watcher_token: StopToken,
        timed_out: Arc<AtomicBool>,
        timeout_token: Option<StopToken>,
    ) -> JoinHandle<()> {
        thread::spawn(|| {
            let icon = Icon::from_resource(1, None).unwrap();

            let tray_menu = Menu::new();

            let quit_i = MenuItem::new("Quit", true, None);

            let authors = env!("CARGO_PKG_AUTHORS")
                .split(':')
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>();

            tray_menu
                .append_items(&[
                    &PredefinedMenuItem::about(
                        None,
                        Some(AboutMetadata {
                            name: Some("Yet Another BG3 Native Mod Loader".to_owned()),
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
                    .with_tooltip("Yet Another BG3 Native Mod Loader")
                    .with_menu(Box::new(tray_menu))
                    .with_icon(icon)
                    .build()
                    .unwrap(),
            );

            EventLoop::new().run(move |event_loop| {
                if let Ok(event) = MenuEvent::receiver().try_recv() {
                    if event.id == quit_i.id() {
                        if let Some(token) = timeout_token.as_ref() {
                            token.stop();
                        }

                        watcher_token.stop();
                        event_loop.exit();

                        tray_icon.take();

                        if timed_out.load(Ordering::Relaxed) {
                            // this may not ever exit, so we have to force it here
                            process::exit(0);
                        }
                    }
                }
            });
        })
    }
}
