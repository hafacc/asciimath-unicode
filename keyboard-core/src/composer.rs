//! Holding what is typed between the math delimiters

use crate::delimiter::Delimiter;
use crate::key::Key;
use unicode_segmentation::UnicodeSegmentation;

/// What the document should show after a key
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// Text to hand to the document in place of what was held
    pub committed: String,
    /// The held text to show underlined after `committed`; empty when nothing is held
    pub marked: String,
    /// Whether the app still gets the key itself
    pub pass_through: bool,
}

impl Outcome {
    fn passed_through(committed: String) -> Self {
        Outcome {
            committed,
            marked: String::new(),
            pass_through: true,
        }
    }

    fn handled(committed: String, marked: String) -> Self {
        Outcome {
            committed,
            marked,
            pass_through: false,
        }
    }
}

/// Holds what is typed between the math delimiters and converts it
///
/// Ordinary typing passes through untouched. From the first character of the opening delimiter
/// the typed text is held instead of reaching the document, and [`Outcome::marked`] shows it as
/// the opening delimiter followed by the math converted so far. Typing the closing delimiter hands
/// over the converted math without its delimiters, and so does return once there is math. Escape,
/// any [`Key::Other`] and [`end_input`][Composer::end_input] hand over the held text exactly as
/// typed, and backspace removes its last character.
///
/// The delimiters follow the rules of the extension's `findSpans`: math is at least one
/// character, and a delimiter after an odd number of backslashes is plain text. The delimiter is
/// [`Delimiter::DoubleDollar`] unless set otherwise.
///
/// Every method that may convert takes the function to convert with, which is given the math and
/// whether to show the parts that aren't there yet.
///
/// ```
/// use keyboard_core::{Composer, Key};
/// let convert = |math: &str, _placeholders: bool| math.to_uppercase();
/// let mut composer = Composer::default();
/// assert_eq!(composer.press(Key::Text("$"), &convert).marked, "$");
/// assert_eq!(composer.press(Key::Text("$"), &convert).marked, "$$");
/// assert_eq!(composer.press(Key::Text("x"), &convert).marked, "$$X");
/// assert_eq!(composer.press(Key::Text("$"), &convert).marked, "$$X$");
/// assert_eq!(composer.press(Key::Text("$"), &convert).committed, "X");
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Composer {
    delimiter: Delimiter,
    /// Everything typed since the first character of the opening delimiter, as typed
    held: String,
    /// How many backslashes are known to sit right before `held`
    backslashes_before: usize,
}

/// A function that converts math to unicode, showing missing parts when its flag is set
pub type Convert<'a> = &'a dyn Fn(&str, bool) -> String;

fn trailing_backslashes(text: &str) -> usize {
    text.chars().rev().take_while(|&chr| chr == '\\').count()
}

impl Composer {
    /// A composer holding nothing, with the given delimiter
    #[must_use]
    pub fn new(delimiter: Delimiter) -> Self {
        Composer {
            delimiter,
            ..Composer::default()
        }
    }

    /// The delimiter that starts and ends math
    #[must_use]
    pub fn delimiter(&self) -> Delimiter {
        self.delimiter
    }

    /// Whether any typed text is held, even the first half of an opening delimiter
    #[must_use]
    pub fn is_holding(&self) -> bool {
        !self.held.is_empty()
    }

    /// Change the delimiter, unless text is held, since that text was read with the old one
    pub fn set_delimiter(&mut self, delimiter: Delimiter) {
        if !self.is_holding() {
            self.delimiter = delimiter;
        }
    }

    /// Take one key and return what the document should now show
    ///
    /// While nothing is held and `key` doesn't start a delimiter, the outcome is empty with
    /// `pass_through` set, so the app handles the key as if the keyboard weren't there. Otherwise:
    ///
    /// - [`Key::Text`] is added to the held text, or handed back in `committed` if it ended the
    ///   holding, so that it lands after what was held. It is never passed through as well.
    /// - [`Key::Backspace`] removes the last held character, back through the opening delimiter.
    /// - [`Key::Escape`] hands over the held text as typed and is not passed through.
    /// - [`Key::Enter`] hands over the held math converted and is not passed through; with no
    ///   math held it acts as [`Key::Other`].
    /// - [`Key::Other`] hands over the held text as typed and is passed through.
    ///
    /// [`Outcome::marked`] is the held text to show after the key, converted with placeholders.
    pub fn press(&mut self, key: Key<'_>, convert: Convert<'_>) -> Outcome {
        match key {
            Key::Text(text) => {
                let was_holding = self.is_holding();
                let mut committed = String::new();
                for character in text.graphemes(true) {
                    self.type_character(character, &mut committed, convert);
                }
                if !was_holding && !self.is_holding() && committed == text {
                    Outcome::passed_through(String::new())
                } else {
                    Outcome::handled(committed, self.marked(convert))
                }
            }
            Key::Backspace => {
                if let Some((start, _)) = self.held.grapheme_indices(true).next_back() {
                    self.held.truncate(start);
                    Outcome::handled(String::new(), self.marked(convert))
                } else {
                    self.backslashes_before = self.backslashes_before.saturating_sub(1);
                    Outcome::passed_through(String::new())
                }
            }
            Key::Escape => {
                if self.is_holding() {
                    let mut committed = String::new();
                    self.flush(&mut committed);
                    Outcome::handled(committed, String::new())
                } else {
                    Outcome::passed_through(String::new())
                }
            }
            Key::Enter => {
                let content = self.content();
                let math = &content[..content.len() - self.closer_tail().len()];
                if math.is_empty() {
                    Outcome::passed_through(self.end_input())
                } else {
                    let converted = convert(math, false);
                    self.remember(&converted);
                    self.held.clear();
                    Outcome::handled(converted, String::new())
                }
            }
            Key::Other => Outcome::passed_through(self.end_input()),
        }
    }

    /// Stop holding and return the held text as typed, for when the caret or focus has moved
    pub fn end_input(&mut self) -> String {
        let mut committed = String::new();
        self.flush(&mut committed);
        self.backslashes_before = 0;
        committed
    }

    /// The name being typed at the end of the held math, from an emoji name's colon
    ///
    /// ```
    /// use keyboard_core::{Composer, Key};
    /// let convert = |math: &str, _placeholders: bool| math.to_owned();
    /// let mut composer = Composer::default();
    /// composer.press(Key::Text("$$x :sm"), &convert);
    /// assert_eq!(composer.partial_name(), Some(":sm"));
    /// ```
    #[must_use]
    pub fn partial_name(&self) -> Option<&str> {
        let math = self.math()?;
        crate::completion::partial_name_start(math).map(|start| &math[start..])
    }

    /// Replace the name being typed with `name`, as if the rest of it had been typed
    ///
    /// With no name being typed, nothing changes.
    pub fn complete(&mut self, name: &str, convert: Convert<'_>) -> Outcome {
        let mut committed = String::new();
        if let Some(start) = self.math().and_then(crate::completion::partial_name_start) {
            self.held.truncate(self.delimiter.open().len() + start);
            for character in name.graphemes(true) {
                self.type_character(character, &mut committed, convert);
            }
        }
        Outcome::handled(committed, self.marked(convert))
    }

    /// The held math, when the opening delimiter is whole and no closing one is begun
    fn math(&self) -> Option<&str> {
        (!self.is_pending_opener() && self.closer_tail().is_empty()).then(|| self.content())
    }

    fn marked(&self, convert: Convert<'_>) -> String {
        if self.is_pending_opener() {
            self.held.clone()
        } else {
            let content = self.content();
            let tail = self.closer_tail();
            let math = &content[..content.len() - tail.len()];
            format!("{}{}{tail}", self.delimiter.open(), convert(math, true))
        }
    }

    fn content(&self) -> &str {
        self.held.get(self.delimiter.open().len()..).unwrap_or("")
    }

    // only characters that continue the opening delimiter are held before it is whole, so being
    // shorter than it is being a prefix of it
    fn is_pending_opener(&self) -> bool {
        self.held.len() < self.delimiter.open().len()
    }

    /// The start of a closing delimiter at the end of the content, which is shown as typed
    fn closer_tail(&self) -> &'static str {
        let close = self.delimiter.close();
        (1..close.len())
            .rev()
            .filter_map(|len| close.get(..len))
            .find(|tail| self.closes(tail))
            .unwrap_or("")
    }

    /// Whether the content is at least one character followed by an unescaped `closer`
    fn closes(&self, closer: &str) -> bool {
        let content = self.content();
        content.len() > closer.len()
            && content.ends_with(closer)
            && trailing_backslashes(&self.held[..self.held.len() - closer.len()]).is_multiple_of(2)
    }

    fn type_character(&mut self, character: &str, committed: &mut String, convert: Convert<'_>) {
        let open = self.delimiter.open();
        if self.held.is_empty() {
            if open.starts_with(character) && self.backslashes_before.is_multiple_of(2) {
                self.held.push_str(character);
            } else {
                self.remember(character);
                committed.push_str(character);
            }
        } else if self.is_pending_opener()
            && !open
                .strip_prefix(self.held.as_str())
                .is_some_and(|rest| rest.starts_with(character))
        {
            self.flush(committed);
            self.type_character(character, committed, convert);
        } else {
            self.held.push_str(character);
            let close = self.delimiter.close();
            if self.closes(close) {
                let content = self.content();
                let converted = convert(&content[..content.len() - close.len()], false);
                committed.push_str(&converted);
                self.remember(&converted);
                self.held.clear();
            }
        }
    }

    /// Hand over the held text as typed
    fn flush(&mut self, committed: &mut String) {
        committed.push_str(&self.held);
        let held = std::mem::take(&mut self.held);
        self.remember(&held);
    }

    fn remember(&mut self, text: &str) {
        let trailing = trailing_backslashes(text);
        self.backslashes_before = if trailing == text.chars().count() {
            self.backslashes_before + trailing
        } else {
            trailing
        };
    }
}

#[cfg(test)]
mod tests {
    use super::{Composer, Outcome};
    use crate::delimiter::Delimiter;
    use crate::key::Key;

    /// Marks what it converted: `«x»` for final text, `‹x›` for text being typed
    fn mark(math: &str, placeholders: bool) -> String {
        if placeholders {
            format!("‹{math}›")
        } else {
            format!("«{math}»")
        }
    }

    fn press(composer: &mut Composer, key: Key<'_>) -> Outcome {
        composer.press(key, &mark)
    }

    fn passed() -> Outcome {
        Outcome::passed_through(String::new())
    }

    fn marked(marked: &str) -> Outcome {
        Outcome::handled(String::new(), marked.to_owned())
    }

    fn committed(committed: &str) -> Outcome {
        Outcome::handled(committed.to_owned(), String::new())
    }

    /// What is in the document and what is still held after typing keys one at a time
    #[derive(Debug, Default, PartialEq, Eq)]
    struct Typed {
        document: String,
        marked: String,
    }

    fn typed(document: &str, marked: &str) -> Typed {
        Typed {
            document: document.to_owned(),
            marked: marked.to_owned(),
        }
    }

    fn type_keys(keys: &[Key<'_>], composer: &mut Composer) -> Typed {
        let mut typed = Typed::default();
        for &key in keys {
            let outcome = press(composer, key);
            typed.document.push_str(&outcome.committed);
            typed.marked = outcome.marked;
            if outcome.pass_through {
                match key {
                    Key::Text(text) => typed.document.push_str(text),
                    Key::Backspace => {
                        typed.document.pop();
                    }
                    Key::Enter => typed.document.push('\n'),
                    Key::Escape | Key::Other => {}
                }
            }
        }
        typed
    }

    /// One key per character, with return for a newline
    fn keys(text: &str) -> Vec<Key<'_>> {
        text.char_indices()
            .map(|(start, chr)| {
                if chr == '\n' {
                    Key::Enter
                } else {
                    Key::Text(&text[start..start + chr.len_utf8()])
                }
            })
            .collect()
    }

    fn type_with(text: &str, delimiter: Delimiter) -> Typed {
        type_keys(&keys(text), &mut Composer::new(delimiter))
    }

    fn type_text(text: &str) -> Typed {
        type_with(text, Delimiter::DoubleDollar)
    }

    #[test]
    fn ordinary_typing_passes_through() {
        let mut composer = Composer::default();
        for key in keys("plain text, with (brackets) and \\ and `") {
            assert_eq!(press(&mut composer, key), passed());
        }
        assert_eq!(press(&mut composer, Key::Backspace), passed());
        assert_eq!(press(&mut composer, Key::Escape), passed());
        assert_eq!(press(&mut composer, Key::Enter), passed());
        assert_eq!(press(&mut composer, Key::Other), passed());
        assert!(!composer.is_holding());
    }

    #[test]
    fn first_half_of_the_opener_is_held() {
        let mut composer = Composer::default();
        assert_eq!(press(&mut composer, Key::Text("$")), marked("$"));
        assert!(composer.is_holding());
        assert_eq!(press(&mut composer, Key::Text("5")), committed("$5"));
        assert!(!composer.is_holding());
    }

    #[test]
    fn typing_between_delimiters_is_held_and_shown_converted() {
        let mut composer = Composer::default();
        assert_eq!(type_keys(&keys("a $$"), &mut composer), typed("a ", "$$‹›"));
        assert_eq!(press(&mut composer, Key::Text("x")), marked("$$‹x›"));
        assert_eq!(press(&mut composer, Key::Text("^")), marked("$$‹x^›"));
        assert_eq!(press(&mut composer, Key::Text("2")), marked("$$‹x^2›"));
    }

    #[test]
    fn closing_delimiter_hands_over_the_final_text() {
        let mut composer = Composer::default();
        type_keys(&keys("$$x^2"), &mut composer);
        assert_eq!(press(&mut composer, Key::Text("$")), marked("$$‹x^2›$"));
        assert_eq!(press(&mut composer, Key::Text("$")), committed("«x^2»"));
        assert!(!composer.is_holding());
        assert_eq!(
            type_text("see $$x^2$$ and $$y$$."),
            typed("see «x^2» and «y».", "")
        );
    }

    #[test]
    fn half_a_closer_followed_by_other_text_is_math() {
        assert_eq!(type_text("$$a$b"), typed("", "$$‹a$b›"));
        assert_eq!(type_text("$$a$b$$"), typed("«a$b»", ""));
    }

    #[test]
    fn math_is_at_least_one_character() {
        assert_eq!(type_text("$$$$"), typed("", "$$‹$›$"));
        assert_eq!(type_text("$$$$$"), typed("«$»", ""));
        assert_eq!(type_with("``", Delimiter::Backtick), typed("", "`‹`›"));
        assert_eq!(type_with("``x`", Delimiter::Backtick), typed("«`x»", ""));
    }

    #[test]
    fn escape_hands_over_what_was_typed() {
        let mut composer = Composer::default();
        type_keys(&keys("$$x^2"), &mut composer);
        assert_eq!(press(&mut composer, Key::Escape), committed("$$x^2"));
        assert!(!composer.is_holding());
        assert_eq!(press(&mut composer, Key::Text("$")), marked("$"));
        assert_eq!(press(&mut composer, Key::Escape), committed("$"));
    }

    #[test]
    fn backspace_undoes_one_key_at_a_time() {
        let mut composer = Composer::default();
        type_keys(&keys("$$xy$"), &mut composer);
        assert_eq!(press(&mut composer, Key::Backspace), marked("$$‹xy›"));
        assert_eq!(press(&mut composer, Key::Backspace), marked("$$‹x›"));
        assert_eq!(press(&mut composer, Key::Backspace), marked("$$‹›"));
        assert_eq!(press(&mut composer, Key::Backspace), marked("$"));
        assert_eq!(press(&mut composer, Key::Backspace), marked(""));
        assert!(!composer.is_holding());
        assert_eq!(press(&mut composer, Key::Backspace), passed());
    }

    #[test]
    fn backspaced_opener_can_be_typed_again() {
        let mut composer = Composer::default();
        let mut all = keys("$$x");
        all.extend([Key::Backspace, Key::Backspace]);
        all.extend(keys("$y$$"));
        assert_eq!(type_keys(&all, &mut composer), typed("«y»", ""));
    }

    #[test]
    fn backspace_removes_a_whole_character() {
        let mut composer = Composer::default();
        type_keys(&keys("$$x"), &mut composer);
        press(&mut composer, Key::Text("👍🏽"));
        assert_eq!(press(&mut composer, Key::Backspace), marked("$$‹x›"));
    }

    #[test]
    fn return_hands_over_the_final_text() {
        let mut composer = Composer::default();
        type_keys(&keys("$$x^2"), &mut composer);
        assert_eq!(press(&mut composer, Key::Enter), committed("«x^2»"));
        assert!(!composer.is_holding());
        assert_eq!(type_text("$$x$\ny"), typed("«x»y", ""));
    }

    #[test]
    fn return_without_math_ends_the_line() {
        let mut composer = Composer::default();
        type_keys(&keys("$$"), &mut composer);
        assert_eq!(
            press(&mut composer, Key::Enter),
            Outcome::passed_through("$$".to_owned())
        );
        assert!(!composer.is_holding());
    }

    #[test]
    fn other_keys_leave_the_text_as_typed() {
        let mut composer = Composer::default();
        type_keys(&keys("$$x^2"), &mut composer);
        assert_eq!(
            press(&mut composer, Key::Other),
            Outcome::passed_through("$$x^2".to_owned())
        );
        press(&mut composer, Key::Text("$"));
        assert_eq!(
            press(&mut composer, Key::Other),
            Outcome::passed_through("$".to_owned())
        );
    }

    #[test]
    fn ending_input_leaves_the_text_as_typed() {
        let mut composer = Composer::default();
        type_keys(&keys("$$x^2$"), &mut composer);
        assert_eq!(composer.end_input(), "$$x^2$");
        assert!(!composer.is_holding());
        assert_eq!(composer.end_input(), "");
    }

    #[test]
    fn escaped_delimiters_are_plain_text() {
        assert_eq!(type_text("\\$$"), typed("\\$", "$"));
        assert_eq!(type_text("\\$$x$$"), typed("\\$$x", "$$‹›"));
        assert_eq!(type_text("\\\\$$x$$"), typed("\\\\«x»", ""));
        assert_eq!(type_text("$$a\\$$b"), typed("", "$$‹a\\$$b›"));
        assert_eq!(type_text("$$a\\$$b$$"), typed("«a\\$$b»", ""));
        assert_eq!(type_text("$$a\\\\$$"), typed("«a\\\\»", ""));
    }

    #[test]
    fn backslash_delimiters() {
        let paren = |text| type_with(text, Delimiter::Paren);
        assert_eq!(paren("\\(x^2"), typed("", "\\(‹x^2›"));
        assert_eq!(paren("\\(x^2\\"), typed("", "\\(‹x^2›\\"));
        assert_eq!(paren("\\(x^2\\)"), typed("«x^2»", ""));
        assert_eq!(
            type_with("a\\ b \\[x\\] c", Delimiter::Bracket),
            typed("a\\ b «x» c", "")
        );
        // the backslash of the closer is itself escaped
        assert_eq!(paren("\\(x\\\\)"), typed("", "\\(‹x\\\\)›"));
        assert_eq!(paren("\\\\(x"), typed("\\\\(x", ""));
        assert_eq!(paren("\\\\\\(x"), typed("\\\\", "\\(‹x›"));
        assert_eq!(paren("\\(a\\ b\\)"), typed("«a\\ b»", ""));
    }

    #[test]
    fn backtick_delimiter() {
        assert_eq!(type_with("`", Delimiter::Backtick), typed("", "`‹›"));
        assert_eq!(
            type_with("a `x^2` b", Delimiter::Backtick),
            typed("a «x^2» b", "")
        );
    }

    #[test]
    fn several_characters_in_one_key() {
        let mut composer = Composer::default();
        assert_eq!(press(&mut composer, Key::Text("ab")), passed());
        assert_eq!(press(&mut composer, Key::Text("$$x")), marked("$$‹x›"));
        assert_eq!(press(&mut composer, Key::Text("$$ok")), committed("«x»ok"));
    }

    #[test]
    fn backslashes_before_the_caret_are_tracked() {
        let mut composer = Composer::default();
        let mut all = keys("\\\\");
        all.push(Key::Backspace);
        all.extend(keys("$"));
        assert_eq!(type_keys(&all, &mut composer), typed("\\$", ""));
        press(&mut composer, Key::Other);
        assert_eq!(press(&mut composer, Key::Text("$")), marked("$"));
    }

    #[test]
    fn delimiter_only_changes_while_nothing_is_held() {
        let mut composer = Composer::default();
        press(&mut composer, Key::Text("$"));
        composer.set_delimiter(Delimiter::Backtick);
        assert_eq!(composer.delimiter(), Delimiter::DoubleDollar);
        press(&mut composer, Key::Escape);
        composer.set_delimiter(Delimiter::Backtick);
        assert_eq!(composer.delimiter(), Delimiter::Backtick);
        assert_eq!(press(&mut composer, Key::Text("`")), marked("`‹›"));
    }

    #[test]
    fn completing_replaces_the_name_being_typed() {
        let mut composer = Composer::default();
        press(&mut composer, Key::Text("$$x :ca"));
        assert_eq!(composer.partial_name(), Some(":ca"));
        assert_eq!(
            composer.complete(":smiley_cat:", &mark),
            marked("$$‹x :smiley_cat:›")
        );
        assert_eq!(composer.partial_name(), None);
        assert_eq!(
            composer.complete(":cat:", &mark),
            marked("$$‹x :smiley_cat:›")
        );
    }

    #[test]
    fn no_name_outside_the_math() {
        let mut composer = Composer::default();
        assert_eq!(composer.partial_name(), None);
        press(&mut composer, Key::Text("$"));
        assert_eq!(composer.partial_name(), None);
        press(&mut composer, Key::Text("$:sm$"));
        assert_eq!(composer.partial_name(), None);
        press(&mut composer, Key::Backspace);
        assert_eq!(composer.partial_name(), Some(":sm"));
    }
}
