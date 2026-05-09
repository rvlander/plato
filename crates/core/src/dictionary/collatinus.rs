use std::ffi::{CString, CStr};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use super::collatinus_sys;
use super::Dictionary;
use super::DictError;

pub struct CollatinusDictionary {
    lang: CString,
    ready: Arc<AtomicBool>,
}

impl CollatinusDictionary {
    pub fn new(lang: &str, ready: Arc<AtomicBool>) -> Self {
        CollatinusDictionary {
            lang: CString::new(lang).expect("lang must not contain null bytes"),
            ready,
        }
    }
}

/// Call from a background thread to warm up Collatinus before the user needs it.
/// The caller must set the ready flag after this returns.
pub fn preload(lang: &str) {
    if let Ok(c_lang) = CString::new(lang) {
        unsafe { collatinus_sys::collatinus_init(c_lang.as_ptr()); }
    }
}

impl Dictionary for CollatinusDictionary {
    fn lookup(&mut self, word: &str, _fuzzy: bool) -> Result<Vec<[String; 2]>, DictError> {
        if !self.ready.load(Ordering::Acquire) {
            return Ok(vec![
                ["".to_string(),
                 "<p class=\"info\">Latin dictionary is loading…</p>".to_string()],
            ]);
        }
        let c_word = match CString::new(word) {
            Ok(s) => s,
            Err(_) => return Ok(vec![]),
        };
        let raw = unsafe {
            collatinus_sys::collatinus_lookup(c_word.as_ptr(), self.lang.as_ptr())
        };
        if raw.is_null() {
            return Ok(vec![]);
        }
        let html = unsafe {
            let s = CStr::from_ptr(raw).to_string_lossy().into_owned();
            collatinus_sys::collatinus_free_result(raw);
            s
        };
        Ok(vec![[word.to_string(), html]])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, atomic::AtomicBool};
    use crate::dictionary::Dictionary;

    fn _assert_impl_dictionary(_: &dyn Dictionary) {}

    #[test]
    fn collatinus_dictionary_implements_trait() {
        let ready = Arc::new(AtomicBool::new(false));
        let d = CollatinusDictionary::new("fr", Arc::clone(&ready));
        _assert_impl_dictionary(&d);
    }

    #[test]
    fn lookup_returns_loading_placeholder_when_not_ready() {
        let ready = Arc::new(AtomicBool::new(false));
        let mut d = CollatinusDictionary::new("fr", Arc::clone(&ready));
        let result = d.lookup("amor", false).unwrap();
        assert_eq!(result.len(), 1);
        assert!(result[0][1].contains("loading"));
    }
}
