//! The C interface, declared for C in `include/asciimath_core.h`

use crate::{Completion, Composer, Delimiter, Key, Outcome, completions, convert};
use std::ffi::{CStr, CString, c_char};
use std::ptr;

/// How math is read and converted, as `asciimath_core.h` declares it
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct AsciimathOptions {
    /// The delimiter's number, as [`Delimiter::from_number`] reads it
    pub delimiter: u8,
    /// Drop ( ), [ ] and { } that only group a fraction, script or argument
    pub strip_brackets: bool,
    /// Write fractions that have a character of their own, like ½, with it
    pub vulgar_fracs: bool,
    /// Write other fractions as a superscript over a subscript, like ʸ⁄ₓ
    pub script_fracs: bool,
    /// The skin tone for emoji that take one, where 0 and unknown numbers are none
    pub skin_tone: u8,
}

impl AsciimathOptions {
    fn convert(self, math: &str, placeholders: bool) -> String {
        convert(
            math,
            self.strip_brackets,
            self.vulgar_fracs,
            self.script_fracs,
            self.skin_tone,
            placeholders,
        )
    }
}

/// An [`Outcome`] for C, whose strings are released together with [`asciimath_outcome_free`]
#[derive(Debug)]
#[repr(C)]
pub struct AsciimathOutcome {
    /// Text to hand to the document in place of what was held
    pub committed: *mut c_char,
    /// The held text to show underlined after `committed`; empty when nothing is held
    pub marked: *mut c_char,
    /// Whether the app still gets the key itself
    pub pass_through: bool,
}

/// A C string of `text`, or an empty one if `text` has a nul, which can't be typed
fn c_string(text: String) -> *mut c_char {
    CString::new(text).unwrap_or_default().into_raw()
}

impl From<Outcome> for AsciimathOutcome {
    fn from(outcome: Outcome) -> Self {
        AsciimathOutcome {
            committed: c_string(outcome.committed),
            marked: c_string(outcome.marked),
            pass_through: outcome.pass_through,
        }
    }
}

/// Create a composer holding nothing, to release with [`asciimath_composer_free`]
#[unsafe(no_mangle)]
pub extern "C" fn asciimath_composer_new() -> *mut Composer {
    Box::into_raw(Box::default())
}

/// Release a composer from [`asciimath_composer_new`]
///
/// # Safety
///
/// `composer` must be null or a pointer from [`asciimath_composer_new`] that hasn't been released.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asciimath_composer_free(composer: *mut Composer) {
    if !composer.is_null() {
        // SAFETY: the caller guarantees `composer` came from `Box::into_raw` and is unreleased
        drop(unsafe { Box::from_raw(composer) });
    }
}

/// Whether any typed text is held, even the first half of an opening delimiter
///
/// # Safety
///
/// `composer` must be null or point to a live composer that nothing else is using.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asciimath_composer_is_holding(composer: *const Composer) -> bool {
    // SAFETY: the caller guarantees a non-null `composer` is live and unshared
    unsafe { composer.as_ref() }.is_some_and(Composer::is_holding)
}

/// Take the key of a macOS key-down event and return what the document should now show
///
/// This is [`Composer::press`] with the key read by [`Key::from_mac`], after setting the
/// delimiter in `options` if nothing is held. A null `composer`, or `characters` that are null or
/// not utf-8, pass the key through.
///
/// # Safety
///
/// `composer` must be null or point to a live composer that nothing else is using, and
/// `characters` must be null or point to a nul-terminated string that stays valid for the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asciimath_composer_press_mac(
    composer: *mut Composer,
    characters: *const c_char,
    key_code: u16,
    is_shortcut: bool,
    options: AsciimathOptions,
) -> AsciimathOutcome {
    // SAFETY: the caller guarantees a non-null `composer` is live and unshared
    let composer = unsafe { composer.as_mut() };
    let characters = if characters.is_null() {
        None
    } else {
        // SAFETY: the caller guarantees a non-null `characters` is a valid nul-terminated string
        unsafe { CStr::from_ptr(characters) }.to_str().ok()
    };
    let outcome = match (composer, characters) {
        (Some(composer), Some(characters)) => {
            composer.set_delimiter(Delimiter::from_number(options.delimiter));
            composer.press(
                Key::from_mac(characters, key_code, is_shortcut),
                &|math, placeholders| options.convert(math, placeholders),
            )
        }
        _ => Outcome {
            pass_through: true,
            ..Outcome::default()
        },
    };
    outcome.into()
}

/// A [`Completion`] for C
#[derive(Debug)]
#[repr(C)]
pub struct AsciimathCompletion {
    /// The whole name as it is typed, like `:smile:`
    pub name: *mut c_char,
    /// What the name converts to, like 😄
    pub symbol: *mut c_char,
}

/// Completions for C, best first, released together with [`asciimath_completions_free`]
#[derive(Debug)]
#[repr(C)]
pub struct AsciimathCompletions {
    /// The first of `count` completions, or null when there are none
    pub items: *mut AsciimathCompletion,
    /// How many completions `items` holds
    pub count: usize,
}

impl From<Vec<Completion>> for AsciimathCompletions {
    fn from(completions: Vec<Completion>) -> Self {
        let items: Box<[AsciimathCompletion]> = completions
            .into_iter()
            .map(|completion| AsciimathCompletion {
                name: c_string(completion.name),
                symbol: c_string(completion.symbol),
            })
            .collect();
        let count = items.len();
        AsciimathCompletions {
            items: if count == 0 {
                ptr::null_mut()
            } else {
                Box::into_raw(items).cast()
            },
            count,
        }
    }
}

/// The names that finish the one being typed at the end of the held math, best first
///
/// This is [`completions`] of [`Composer::partial_name`], converting with `options`. There are
/// none when no name is being typed or `composer` is null.
///
/// # Safety
///
/// `composer` must be null or point to a live composer that nothing else is using.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asciimath_composer_completions(
    composer: *const Composer,
    options: AsciimathOptions,
) -> AsciimathCompletions {
    // SAFETY: the caller guarantees a non-null `composer` is live and unshared
    unsafe { composer.as_ref() }
        .and_then(Composer::partial_name)
        .map_or_else(Vec::new, |partial| {
            completions(partial, &|math, placeholders| {
                options.convert(math, placeholders)
            })
        })
        .into()
}

/// Release completions from [`asciimath_composer_completions`]
///
/// # Safety
///
/// `completions` must be ones returned by [`asciimath_composer_completions`] that haven't been
/// released yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asciimath_completions_free(completions: AsciimathCompletions) {
    if !completions.items.is_null() {
        // SAFETY: the caller guarantees `items` and `count` came from a boxed slice that is
        // unreleased, and so are the strings in it
        let items = unsafe {
            Box::from_raw(ptr::slice_from_raw_parts_mut(
                completions.items,
                completions.count,
            ))
        };
        for item in items {
            // SAFETY: as above
            unsafe {
                asciimath_free(item.name);
                asciimath_free(item.symbol);
            }
        }
    }
}

/// Replace the name being typed with `name` and return what the document should now show
///
/// This is [`Composer::complete`], converting with `options`. A null `composer`, or a `name`
/// that is null or not utf-8, changes nothing and shows nothing new.
///
/// # Safety
///
/// `composer` must be null or point to a live composer that nothing else is using, and `name`
/// must be null or point to a nul-terminated string that stays valid for the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asciimath_composer_complete(
    composer: *mut Composer,
    name: *const c_char,
    options: AsciimathOptions,
) -> AsciimathOutcome {
    // SAFETY: the caller guarantees a non-null `composer` is live and unshared
    let composer = unsafe { composer.as_mut() };
    let name = if name.is_null() {
        None
    } else {
        // SAFETY: the caller guarantees a non-null `name` is a valid nul-terminated string
        unsafe { CStr::from_ptr(name) }.to_str().ok()
    };
    match (composer, name) {
        (Some(composer), Some(name)) => composer
            .complete(name, &|math, placeholders| {
                options.convert(math, placeholders)
            })
            .into(),
        _ => Outcome::default().into(),
    }
}

/// Stop holding and return the held text as typed, to release with [`asciimath_free`]
///
/// # Safety
///
/// `composer` must be null or point to a live composer that nothing else is using.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asciimath_composer_end_input(composer: *mut Composer) -> *mut c_char {
    // SAFETY: the caller guarantees a non-null `composer` is live and unshared
    c_string(unsafe { composer.as_mut() }.map_or_else(String::new, Composer::end_input))
}

/// Release the strings of an outcome from [`asciimath_composer_press_mac`]
///
/// # Safety
///
/// `outcome` must be one returned by [`asciimath_composer_press_mac`] whose strings haven't been
/// released yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asciimath_outcome_free(outcome: AsciimathOutcome) {
    // SAFETY: the caller guarantees both strings came from this library and are unreleased
    unsafe {
        asciimath_free(outcome.committed);
        asciimath_free(outcome.marked);
    }
}

/// Release a string returned by this library
///
/// # Safety
///
/// `text` must be null or a string this library returned that hasn't been released yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asciimath_free(text: *mut c_char) {
    if !text.is_null() {
        // SAFETY: the caller guarantees `text` came from `CString::into_raw` and is unreleased
        drop(unsafe { CString::from_raw(text) });
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AsciimathOptions, asciimath_completions_free, asciimath_composer_complete,
        asciimath_composer_completions, asciimath_composer_end_input, asciimath_composer_free,
        asciimath_composer_is_holding, asciimath_composer_new, asciimath_composer_press_mac,
        asciimath_free, asciimath_outcome_free,
    };
    use crate::Composer;
    use std::ffi::{CStr, CString, c_char};
    use std::ptr;

    const OPTIONS: AsciimathOptions = AsciimathOptions {
        delimiter: 0,
        strip_brackets: true,
        vulgar_fracs: true,
        script_fracs: true,
        skin_tone: 0,
    };

    /// Take ownership of a string this library returned
    fn owned(text: *mut c_char) -> String {
        // SAFETY: `text` is a string this library just returned, read once and released once
        unsafe {
            let owned = CStr::from_ptr(text).to_str().unwrap().to_owned();
            asciimath_free(text);
            owned
        }
    }

    /// Type `text` a character at a time, returning what was committed and what is still marked
    fn type_text(
        composer: *mut Composer,
        text: &str,
        options: AsciimathOptions,
    ) -> (String, String) {
        let mut committed = String::new();
        let mut marked = String::new();
        for chr in text.chars() {
            let characters = CString::new(chr.to_string()).unwrap();
            // SAFETY: `composer` is live and unshared, `characters` is a valid string, and the
            // outcome's strings are read once and released once
            unsafe {
                let outcome =
                    asciimath_composer_press_mac(composer, characters.as_ptr(), 0, false, options);
                committed.push_str(CStr::from_ptr(outcome.committed).to_str().unwrap());
                marked = CStr::from_ptr(outcome.marked).to_str().unwrap().to_owned();
                if outcome.pass_through {
                    committed.push(chr);
                }
                asciimath_outcome_free(outcome);
            }
        }
        (committed, marked)
    }

    #[test]
    fn composes_through_c() {
        let composer = asciimath_composer_new();
        assert_eq!(
            type_text(composer, "a $$1/2", OPTIONS),
            ("a ".to_owned(), "$$½".to_owned())
        );
        // SAFETY: `composer` is live and unshared
        assert!(unsafe { asciimath_composer_is_holding(composer) });
        assert_eq!(
            type_text(composer, " + x^$$", OPTIONS),
            ("½ + x".to_owned(), String::new())
        );
        assert_eq!(
            type_text(composer, "$$x^", OPTIONS),
            (String::new(), "$$x⸋".to_owned())
        );
        // SAFETY: `composer` is live and unshared, and is released once
        unsafe {
            assert_eq!(owned(asciimath_composer_end_input(composer)), "$$x^");
            assert!(!asciimath_composer_is_holding(composer));
            asciimath_composer_free(composer);
        }
    }

    #[test]
    fn options_are_passed_on() {
        let composer = asciimath_composer_new();
        let plain = AsciimathOptions {
            script_fracs: false,
            ..OPTIONS
        };
        assert_eq!(type_text(composer, "$$y/x$$", OPTIONS).0, "ʸ⁄ₓ");
        assert_eq!(type_text(composer, "$$y/x$$", plain).0, "y/x");
        let dark = AsciimathOptions {
            skin_tone: 5,
            ..OPTIONS
        };
        assert_eq!(type_text(composer, "$$:hand:$$", dark).0, "✋🏿");
        let backtick = AsciimathOptions {
            delimiter: 3,
            ..OPTIONS
        };
        assert_eq!(type_text(composer, "`α^2`", backtick).0, "α²");
        // SAFETY: `composer` is live and unshared, and is released once
        unsafe { asciimath_composer_free(composer) };
    }

    #[test]
    fn invalid_input_passes_through() {
        let composer = asciimath_composer_new();
        let not_utf8 = CString::new([0xff, 0xfe]).unwrap();
        // SAFETY: null is allowed everywhere, `composer` is live and unshared and released once,
        // and each outcome is released once
        unsafe {
            for characters in [ptr::null(), not_utf8.as_ptr()] {
                let outcome = asciimath_composer_press_mac(composer, characters, 0, false, OPTIONS);
                assert!(outcome.pass_through);
                asciimath_outcome_free(outcome);
            }
            let outcome =
                asciimath_composer_press_mac(ptr::null_mut(), c"$".as_ptr(), 0, false, OPTIONS);
            assert!(outcome.pass_through);
            asciimath_outcome_free(outcome);
            assert!(!asciimath_composer_is_holding(ptr::null()));
            assert_eq!(owned(asciimath_composer_end_input(ptr::null_mut())), "");
            asciimath_composer_free(composer);
            asciimath_composer_free(ptr::null_mut());
            asciimath_free(ptr::null_mut());
        }
    }

    /// The names and symbols of the completions offered now
    fn offered(composer: *const Composer, options: AsciimathOptions) -> Vec<(String, String)> {
        // SAFETY: `composer` is live and unshared, `count` items are read, and they are released
        // once
        unsafe {
            let completions = asciimath_composer_completions(composer, options);
            let offered = (0..completions.count)
                .map(|index| {
                    let item = &*completions.items.add(index);
                    (
                        CStr::from_ptr(item.name).to_str().unwrap().to_owned(),
                        CStr::from_ptr(item.symbol).to_str().unwrap().to_owned(),
                    )
                })
                .collect();
            asciimath_completions_free(completions);
            offered
        }
    }

    #[test]
    fn completes_through_c() {
        let composer = asciimath_composer_new();
        assert_eq!(offered(composer, OPTIONS), Vec::new());
        type_text(composer, "$$:han", OPTIONS);
        assert_eq!(
            offered(
                composer,
                AsciimathOptions {
                    skin_tone: 5,
                    ..OPTIONS
                }
            )[0],
            (":hand:".to_owned(), "✋🏿".to_owned())
        );
        // SAFETY: `composer` is live and unshared, the name is a valid string, and the outcome
        // and composer are released once
        unsafe {
            let outcome = asciimath_composer_complete(composer, c":hand:".as_ptr(), OPTIONS);
            assert_eq!(CStr::from_ptr(outcome.marked).to_str().unwrap(), "$$✋");
            asciimath_outcome_free(outcome);
            let outcome = asciimath_composer_complete(ptr::null_mut(), ptr::null(), OPTIONS);
            assert_eq!(CStr::from_ptr(outcome.marked).to_str().unwrap(), "");
            asciimath_outcome_free(outcome);
            asciimath_completions_free(asciimath_composer_completions(ptr::null(), OPTIONS));
            asciimath_composer_free(composer);
        }
    }
}
