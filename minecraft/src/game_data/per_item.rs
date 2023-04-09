
use crate::item::RawItemId;


/// Association of `Option<T>` for each item.
///
/// (Not to be confused with each item instance).
#[derive(Debug, Clone)]
pub struct PerItem<T> {
    vals: Vec<Option<T>>,
    default: Option<T>,
}

impl<T> PerItem<T> {
    pub fn new(default: T) -> Self {
        PerItem {
            vals: Vec::new(),
            default: Some(default),
        }
    }

    pub fn new_no_default() -> Self {
        PerItem {
            vals: Vec::new(),
            default: None,
        }
    }

    pub fn get<I: Into<RawItemId>>(&self, iid: I) -> &T {
        self.vals
            .get(iid.into().0 as usize)
            .and_then(|opt| opt.as_ref())
            .unwrap_or_else(|| self
                .default
                .as_ref()
                .expect("PerItem::get with no val set and no default"))
    }

    pub fn set<I: Into<RawItemId>>(&mut self, iid: I, val: T) {
        let idx = iid.into().0 as usize;
        while self.vals.len() < idx + 1 {
            self.vals.push(None);
        }
        if self.vals[idx].is_some() {
            warn!("overwrite of non-None PerItem value");
        }
        self.vals[idx] = Some(val)
    }
}
