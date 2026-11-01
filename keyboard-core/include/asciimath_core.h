#ifndef ASCIIMATH_CORE_H
#define ASCIIMATH_CORE_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

/* Skin tones, numbered as `AsciimathOptions.skin_tone` takes them. */
enum {
    ASCIIMATH_TONE_DEFAULT = 0,
    ASCIIMATH_TONE_LIGHT = 1,
    ASCIIMATH_TONE_MEDIUM_LIGHT = 2,
    ASCIIMATH_TONE_MEDIUM = 3,
    ASCIIMATH_TONE_MEDIUM_DARK = 4,
    ASCIIMATH_TONE_DARK = 5,
};

/* Math delimiters, numbered as `AsciimathOptions.delimiter` takes them. */
enum {
    ASCIIMATH_DELIMITER_DOUBLE_DOLLAR = 0,
    ASCIIMATH_DELIMITER_PAREN = 1,
    ASCIIMATH_DELIMITER_BRACKET = 2,
    ASCIIMATH_DELIMITER_BACKTICK = 3,
};

/* Holds what is typed between the math delimiters and converts it. */
typedef struct AsciimathComposer AsciimathComposer;

/* How math is read and converted. Unknown numbers are the default delimiter and no skin tone. */
typedef struct {
    uint8_t delimiter;
    bool strip_brackets;
    bool vulgar_fracs;
    bool script_fracs;
    uint8_t skin_tone;
} AsciimathOptions;

/*
 * What the document should show after a key: `committed` replaces what was held, `marked` is the
 * held text to show underlined after it, and `pass_through` says the app still gets the key.
 * Release with `asciimath_outcome_free`.
 */
typedef struct {
    char *_Nonnull committed;
    char *_Nonnull marked;
    bool pass_through;
} AsciimathOutcome;

/* A composer holding nothing, to release with `asciimath_composer_free`. */
AsciimathComposer *_Nonnull asciimath_composer_new(void);

void asciimath_composer_free(AsciimathComposer *_Nullable composer);

/* Whether any typed text is held, even the first half of an opening delimiter. */
bool asciimath_composer_is_holding(const AsciimathComposer *_Nullable composer);

/*
 * Take the key of a macOS key-down event, by its nul-terminated utf-8 characters, its virtual key
 * code and whether command or control was held. The delimiter in `options` takes effect once
 * nothing is held. A NULL composer, or characters that are NULL or not utf-8, pass the key through.
 */
AsciimathOutcome asciimath_composer_press_mac(AsciimathComposer *_Nullable composer,
                                              const char *_Nullable characters, uint16_t key_code,
                                              bool is_shortcut, AsciimathOptions options);

/* A name that finishes the one being typed: `name` as it is typed, like "alpha", and `symbol`
 * what it converts to. */
typedef struct {
    char *_Nonnull name;
    char *_Nonnull symbol;
} AsciimathCompletion;

/* `count` completions, best first, released together with `asciimath_completions_free`. */
typedef struct {
    AsciimathCompletion *_Nullable items;
    size_t count;
} AsciimathCompletions;

/*
 * The names that finish the one being typed at the end of the held math, best first, each symbol
 * converted with `options`. None when no name is being typed.
 */
AsciimathCompletions asciimath_composer_completions(const AsciimathComposer *_Nullable composer,
                                                    AsciimathOptions options);

void asciimath_completions_free(AsciimathCompletions completions);

/*
 * Replace the name being typed with `name`, a nul-terminated utf-8 string, as if the rest
 * of it had been typed, and return what the document should now show. With no name being typed,
 * nothing changes.
 */
AsciimathOutcome asciimath_composer_complete(AsciimathComposer *_Nullable composer,
                                             const char *_Nullable name, AsciimathOptions options);

/* Stop holding and return the held text as typed, to release with `asciimath_free`. */
char *_Nonnull asciimath_composer_end_input(AsciimathComposer *_Nullable composer);

void asciimath_outcome_free(AsciimathOutcome outcome);

void asciimath_free(char *_Nullable text);

#endif
