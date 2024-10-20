use std::thread::JoinHandle;

use tray_icon::{
    menu::{AboutMetadata, Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem},
    Icon, TrayIcon, TrayIconBuilder,
};
use winit::{
    application::ApplicationHandler,
    event::{StartCause, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    platform::windows::EventLoopBuilderExtWindows,
    window::WindowId,
};

use crate::{process_watcher::ProcessWatcherStopToken, thread_helpers};

pub struct AppTray {
    token: ProcessWatcherStopToken,
    quit_id: MenuId,
    tray_icon: Option<TrayIcon>,
}

impl ApplicationHandler for AppTray {
    fn new_events(&mut self, event_loop: &ActiveEventLoop, _cause: StartCause) {
        if let Ok(event) = MenuEvent::receiver().try_recv() {
            if event.id == self.quit_id {
                self.tray_icon.take();
                self.token.stop();
                event_loop.exit();
            }
        }
    }

    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {}

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        _event: WindowEvent,
    ) {
        unimplemented!()
    }
}

impl AppTray {
    pub fn start(token: ProcessWatcherStopToken) -> JoinHandle<()> {
        thread_helpers::spawn_named("AppTray", || {
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
                            name: Some("Yet Another BG3 Native Mod Loader".to_string()),
                            copyright: Some(format!("Copyright (c) {}", authors.join(", ")).to_string()),
                            version: Some(env!("CARGO_PKG_VERSION").to_string()),
                            authors: Some(authors),
                            license: Some(env!("CARGO_PKG_LICENSE").to_string()),
                            website_label: Some("https://www.nexusmods.com/baldursgate3/mods/3052".to_string()),
                            website: Some(env!("CARGO_PKG_HOMEPAGE").to_string()),
                            comments: Some("THE SOFTWARE IS PROVIDED \"AS IS\", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.".to_string()),
                            ..Default::default()
                        }),
                    ),
                    &PredefinedMenuItem::separator(),
                    &quit_i,
                ])
                .unwrap();

            let tray_icon = Some(
                TrayIconBuilder::new()
                    .with_tooltip("Yet Another BG3 Native Mod Loader")
                    .with_menu(Box::new(tray_menu))
                    .with_icon(icon)
                    .build()
                    .unwrap(),
            );

            let event_loop = EventLoop::builder().with_any_thread(true).build().unwrap();

            let mut tray = Self {
                token,
                quit_id: quit_i.id().clone(),
                tray_icon,
            };

            event_loop.run_app(&mut tray).unwrap();
        })
    }
}
