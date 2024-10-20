use std::thread::{self, JoinHandle};

pub fn spawn_named<F, T>(name: &str, cb: F) -> JoinHandle<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    thread::Builder::new()
        .name(name.to_owned())
        .spawn(cb)
        .expect("failed to spawn thread")
}
