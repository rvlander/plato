#pragma once

#ifdef __cplusplus
extern "C" {
#endif

/*
 * Look up a Latin word and return an HTML morphological analysis.
 * lang: ISO 639-1 output language code (e.g. "fr", "en")
 * Returns a heap-allocated UTF-8 string. Caller must call collatinus_free_result().
 * Returns NULL on failure.
 */
char *collatinus_lookup(const char *word, const char *lang);

/* Free a string returned by collatinus_lookup. */
void collatinus_free_result(char *result);

/*
 * On ARM Linux (Kobo), Qt5Core is not linked at build time.
 * Call this once before any other collatinus_* function to load
 * Qt5Core into the process via dlopen(RTLD_GLOBAL).
 * On other platforms this is a no-op.
 */
void collatinus_preload(void);

#ifdef __cplusplus
}
#endif
