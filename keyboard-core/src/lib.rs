//! What the keyboards share: converting ascii math and holding what is typed between delimiters
//!
//! [`Composer`] takes keys and says what the document should show, and [`convert`] converts as the
//! extension's binding does when that keeps typed spaces and adds none around operators.
//! [`completions`] lists the names that finish one being typed. The
//! [`ffi`] module is the same for C, which the mac keyboard's Swift code calls, and [`Settings`]
//! is what each keyboard stores in its own way.
#![warn(
    clippy::pedantic,
    clippy::undocumented_unsafe_blocks,
    missing_docs,
    unsafe_op_in_unsafe_fn
)]

mod completion;
mod composer;
mod delimiter;
pub mod ffi;
mod key;
mod settings;

pub use completion::{Completion, completions};
pub use composer::{Composer, Convert, Outcome};
pub use delimiter::Delimiter;
pub use key::Key;
pub use settings::Settings;

use asciimath_unicode::{Conf, Layout, Placeholders, SkinTone};

/// Skin tone by its number in `asciimath_core.h`, where an unknown number is no skin tone
fn skin_tone(tone: u8) -> SkinTone {
    match tone {
        1 => SkinTone::Light,
        2 => SkinTone::MediumLight,
        3 => SkinTone::Medium,
        4 => SkinTone::MediumDark,
        5 => SkinTone::Dark,
        _ => SkinTone::Default,
    }
}

/// Convert ascii math to unicode
///
/// All of `inp` is read as math, and every string converts. The options are those of
/// [`asciimath_unicode::Conf`]:
///
/// - `strip_brackets` drops ( ), [ ] and { } that only group a fraction, script or argument
/// - `vulgar_fracs` writes fractions that have a character of their own, like ½, with it
/// - `script_fracs` writes other fractions as a superscript over a subscript, like ʸ⁄ₓ, instead of
///   with a plain slash
/// - `tone` is the skin tone for emoji that take one, numbered as in `asciimath_core.h`; 0 and
///   unknown numbers are no skin tone
/// - `placeholders` shows the parts that aren't there yet as □, ⸋ and ▫, for math that is still
///   being typed
///
/// Whitespace typed between parts of the math is always kept, and none is added around operators.
///
/// ```
/// assert_eq!(keyboard_core::convert("1/2", true, true, true, 0, false), "½");
/// ```
#[must_use]
#[allow(clippy::fn_params_excessive_bools)]
pub fn convert(
    inp: &str,
    strip_brackets: bool,
    vulgar_fracs: bool,
    script_fracs: bool,
    tone: u8,
    placeholders: bool,
) -> String {
    Conf::default()
        .with_strip_brackets(strip_brackets)
        .with_vulgar_fracs(vulgar_fracs)
        .with_skin_tone(skin_tone(tone))
        .with_layout(if script_fracs {
            Layout::InlineScript
        } else {
            Layout::InlinePlain
        })
        .with_placeholders(placeholders.then(Placeholders::default))
        .with_keep_spaces(true)
        .parse(inp)
        .to_string()
}

#[cfg(test)]
mod tests {
    #[test]
    fn unknown_skin_tone_is_none() {
        assert_eq!(super::convert(":hand:", true, true, true, 200, false), "✋");
        assert_eq!(super::convert(":hand:", true, true, true, 5, false), "✋🏿");
    }

    #[test]
    fn typed_spaces_are_kept() {
        assert_eq!(super::convert("a + b", true, true, true, 0, false), "a + b");
        assert_eq!(super::convert("a+b", true, true, true, 0, false), "a+b");
    }

    #[test]
    fn placeholders() {
        assert_eq!(super::convert("x^", true, true, true, 0, true), "x⸋");
        assert_eq!(super::convert("x^", true, true, true, 0, false), "x");
    }
}
