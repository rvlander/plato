use std::ffi::{CString, CStr};
use std::sync::Once;
use super::collatinus_sys;
use super::Dictionary;
use super::DictError;

static PRELOAD: Once = Once::new();

fn ensure_preloaded() {
    PRELOAD.call_once(|| unsafe { collatinus_sys::collatinus_preload(); });
}

pub struct CollatinusDictionary {
    lang: CString,
}

impl CollatinusDictionary {
    pub fn new(lang: &str) -> Self {
        CollatinusDictionary {
            lang: CString::new(lang).expect("lang must not contain null bytes"),
        }
    }
}

impl Dictionary for CollatinusDictionary {
    fn lookup(&mut self, word: &str, _fuzzy: bool) -> Result<Vec<[String; 2]>, DictError> {
        ensure_preloaded();
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
    use crate::dictionary::Dictionary;

    fn _assert_impl_dictionary(_: &dyn Dictionary) {}

    #[test]
    fn collatinus_dictionary_implements_trait() {
        let d = CollatinusDictionary::new("fr");
        _assert_impl_dictionary(&d);
    }
}
