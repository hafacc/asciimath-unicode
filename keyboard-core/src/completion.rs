//! Names to offer while one is half typed

use asciimath_unicode::ASCIIMATH_TOKENS;
use std::collections::HashSet;

/// A name that finishes the one being typed
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Completion {
    /// The whole name as it is typed, like `alpha` or `:smile:`
    pub name: String,
    /// What the name converts to, like α or 😄
    pub symbol: String,
}

/// The most completions offered at once
const MOST_COMPLETIONS: usize = 10;

fn is_emoji_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '_' | '+' | '-')
}

/// Where the name being typed at the end of `math` starts
///
/// An emoji name starts at its colon and is followed by at least one character, and any other
/// name is the letters at the end.
pub(crate) fn partial_name_start(math: &str) -> Option<usize> {
    let before_emoji = math.trim_end_matches(is_emoji_character);
    let before_letters = math.trim_end_matches(|character: char| character.is_ascii_alphabetic());
    match before_emoji.strip_suffix(':') {
        Some(colon) if before_emoji.len() < math.len() => Some(colon.len()),
        _ => (before_letters.len() < math.len()).then_some(before_letters.len()),
    }
}

/// Every name of the same sort as `partial`, an emoji's or a word of math, in its table's order
///
/// Each comes with what it is the same as, so that names of one symbol can be told apart.
fn names_like(partial: &str) -> Vec<(String, &'static str)> {
    if partial.starts_with(':') {
        emojis::iter()
            .flat_map(|emoji| {
                emoji
                    .shortcodes()
                    .map(move |code| (format!(":{code}:"), emoji.as_str()))
            })
            .collect()
    } else {
        ASCIIMATH_TOKENS
            .iter()
            .filter(|entry| {
                entry
                    .spelling
                    .chars()
                    .all(|character| character.is_ascii_alphabetic())
            })
            .map(|entry| {
                (
                    entry.spelling.to_owned(),
                    entry.unicode.unwrap_or(entry.spelling),
                )
            })
            .collect()
    }
}

/// The names that finish `partial`, the name typed so far, best first
///
/// `partial` is an emoji name from its colon, like `:smi`, or the start of any other name, like
/// `alp`. Capitals count, so `alp` offers `alpha` but not `Alpha`. Names that start with it come
/// before those that only contain it, then shorter names first, then in the order of asciimath's or Unicode's table. Names that convert to the same thing are offered once, by the best
/// of them. Each symbol is `convert` of the name, asked to show the parts it still needs.
///
/// ```
/// let convert = |math: &str, placeholders: bool| {
///     keyboard_core::convert(math, true, true, true, 0, placeholders)
/// };
/// let completions = keyboard_core::completions("alp", &convert);
/// assert_eq!(completions[0].name, "alpha");
/// assert_eq!(completions[0].symbol, "α");
/// ```
#[must_use]
pub fn completions(partial: &str, convert: crate::Convert<'_>) -> Vec<Completion> {
    let typed = partial.trim_start_matches(':');
    let mut ranked: Vec<_> = names_like(partial)
        .into_iter()
        .filter_map(|(name, same_as)| {
            let spelling = name.trim_start_matches(':');
            let is_prefix = spelling.starts_with(typed);
            (is_prefix || spelling.contains(typed)).then_some((!is_prefix, name, same_as))
        })
        .collect();
    ranked.sort_by_key(|(is_inner, name, _)| (*is_inner, name.len()));
    let mut seen = HashSet::new();
    ranked
        .into_iter()
        .filter(|&(_, _, same_as)| seen.insert(same_as))
        .take(MOST_COMPLETIONS)
        .map(|(_, name, _)| Completion {
            symbol: convert(&name, true),
            name,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{completions, partial_name_start};
    use std::collections::HashSet;

    fn convert(math: &str, placeholders: bool) -> String {
        crate::convert(math, true, true, true, 5, placeholders)
    }

    fn names(partial: &str) -> Vec<String> {
        completions(partial, &convert)
            .into_iter()
            .map(|completion| completion.name)
            .collect()
    }

    #[test]
    fn finds_the_name_being_typed() {
        assert_eq!(partial_name_start(":sm"), Some(0));
        assert_eq!(partial_name_start("x + :+1"), Some(4));
        assert_eq!(partial_name_start(":smile::ca"), Some(7));
        assert_eq!(partial_name_start("x:sm"), Some(1));
        assert_eq!(partial_name_start("x+alp"), Some(2));
        assert_eq!(partial_name_start("2alp"), Some(1));
        assert_eq!(partial_name_start(":"), None);
        assert_eq!(partial_name_start(":smile:"), None);
        assert_eq!(partial_name_start("x+1"), None);
        assert_eq!(partial_name_start("x "), None);
    }

    #[test]
    fn names_that_start_with_it_come_first() {
        let cat = names(":cat");
        assert_eq!(cat[..2], [":cat:", ":cat2:"]);
        let inner = cat
            .iter()
            .position(|name| !name.starts_with(":cat"))
            .unwrap();
        assert!(cat[inner..].iter().all(|name| !name.starts_with(":cat")));
        assert_eq!(names("alp")[0], "alpha");
    }

    #[test]
    fn each_symbol_once() {
        assert_eq!(names(":thumbsu"), [":thumbsup:"]);
        let symbols: Vec<_> = completions("arr", &convert)
            .into_iter()
            .map(|completion| completion.symbol)
            .collect();
        assert_eq!(symbols.iter().collect::<HashSet<_>>().len(), symbols.len());
    }

    #[test]
    fn math_names_are_only_words() {
        assert!(names("s").iter().all(|name| {
            name.chars()
                .all(|character| character.is_ascii_alphabetic())
        }));
        assert!(names(":s").iter().all(|name| name.starts_with(':')));
    }

    #[test]
    fn capitals_count_and_tones() {
        assert_eq!(names("alp"), ["alpha"]);
        assert_eq!(names("Alp"), ["Alpha"]);
        assert!(!names("del").contains(&"Delta".to_owned()));
        assert_eq!(names("R")[0], "RR");
        assert_eq!(completions(":hand", &convert)[0].symbol, "✋🏿");
        assert_eq!(names(":zzzzz"), Vec::<String>::new());
        assert_eq!(names(":e").len(), 10);
    }
}
