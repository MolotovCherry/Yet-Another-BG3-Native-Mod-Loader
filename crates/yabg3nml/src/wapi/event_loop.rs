use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, MSG, PostQuitMessage, TranslateMessage,
};

pub struct EventLoop;

impl EventLoop {
    pub fn new() -> Self {
        Self
    }

    pub fn run(&self, mut cb: impl FnMut(&Self, &MSG)) {
        let mut msg = MSG::default();

        fn get_msg(msg: &mut MSG) -> bool {
            let res = unsafe { GetMessageW(msg, None, 0, 0) };
            res.as_bool()
        }

        while get_msg(&mut msg) {
            _ = unsafe { TranslateMessage(&msg) };
            unsafe {
                DispatchMessageW(&msg);
            }

            cb(self, &msg);
        }
    }

    pub fn exit(&self) {
        unsafe {
            PostQuitMessage(0);
        }
    }
}
