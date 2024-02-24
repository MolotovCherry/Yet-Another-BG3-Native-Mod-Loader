use std::thread::{self};

use tray_icon::{
    menu::{AboutMetadata, Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    Icon, TrayIconBuilder,
};
use winit::{
    event_loop::{ControlFlow, EventLoopBuilder},
    platform::windows::EventLoopBuilderExtWindows,
};

use crate::process_watcher::ProcessWatcherStopToken;

pub struct AppTray;

impl AppTray {
    pub fn start(watcher: ProcessWatcherStopToken) {
        thread::spawn(move || {
            let icon = Icon::from_resource(1, None).unwrap();

            let tray_menu = Menu::new();

            let quit_i = MenuItem::new("Quit", true, None);
            tray_menu
                .append_items(&[
                    &PredefinedMenuItem::about(
                        None,
                        Some(AboutMetadata {
                            name: Some("Yet Another Bg3 Mod Loader".to_string()),
                            copyright: Some("Copyright (c) Cherry".to_string()),
                            version: Some(env!("CARGO_PKG_VERSION").to_string()),
                            authors: Some(vec!["Cherry".to_string()]),
                            license: Some("MIT".to_string()),
                            website_label: Some("https://www.nexusmods.com/baldursgate3/mods/3052".to_string()),
                            website: Some("https://github.com/MolotovCherry/Yet-Another-BG3-Native-Mod-Loader".to_string()),
                            comments: Some("THE SOFTWARE IS PROVIDED \"AS IS\", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.".to_string()),
                            ..Default::default()
                        }),
                    ),
                    &PredefinedMenuItem::separator(),
                    &quit_i,
                ])
                .unwrap();

            let mut tray_icon = Some(
                TrayIconBuilder::new()
                    .with_tooltip("Yet Another Bg3 Mod Loader")
                    .with_menu(Box::new(tray_menu))
                    .with_icon(icon)
                    .build()
                    .unwrap(),
            );

            let event_loop = EventLoopBuilder::new()
                .with_any_thread(true)
                .build()
                .unwrap();

            event_loop
                .run(move |_event, event_loop| {
                    event_loop.set_control_flow(ControlFlow::Wait);

                    if let Ok(event) = MenuEvent::receiver().try_recv() {
                        if event.id == quit_i.id() {
                            tray_icon.take();
                            watcher.stop();
                        }
                    }
                })
                .unwrap();
        });
    }
}
