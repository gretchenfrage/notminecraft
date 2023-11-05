
use std::{
    fmt::{self, Formatter, Debug},
    sync::Arc,
};
use parking_lot::Mutex;


/// Simplified clipboard handling abstraction that swallows and logs errors.
#[derive(Clone)]
pub struct Clipboard(Option<Arc<Mutex<arboard::Clipboard>>>);

impl Clipboard {
    pub fn new() -> Self {
        Clipboard(arboard::Clipboard::new()
            .map_err(|e| error!(%e, "unable to initialize clipboard"))
            .ok()
            .map(|inner| Arc::new(Mutex::new(inner))))
    }

    pub fn get(&self) -> String {
        self.0.as_ref()
            .and_then(|inner| inner.lock().get_text()
                .map_err(|e| error!(%e, "error getting clipboard text"))
                .ok())
            .unwrap_or_default()
    }

    pub fn set(&self, text: &str) {
        if let Some(ref inner) = self.0 {
            if let Err(e) = inner.lock().set_text(text) {
                error!(%e, "error setting clipboard text");
            }
        }
    }
}

impl Debug for Clipboard {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if self.0.is_some() {
            f.write_str("Clipboard(Some(_))")
        } else {
            f.write_str("Clipboard(None)")
        }
    }
}
