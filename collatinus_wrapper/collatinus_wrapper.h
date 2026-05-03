#pragma once

#ifdef __cplusplus
extern "C" {
#endif

/*
 * Initialize Collatinus for a single target language.
 * lang: ISO 639-1 output language code (e.g. "fr", "en").
 * Must be called before the first collatinus_lookup.
 * Subsequent calls are no-ops (the first language wins).
 * Returns 0 on success, -1 on failure.
 */
int collatinus_init(const char *lang);

/*
 * Look up a Latin word and return an HTML morphological analysis.
 * lang: ISO 639-1 output language code (e.g. "fr", "en")
 * Returns a heap-allocated UTF-8 string. Caller must call collatinus_free_result().
 * Returns NULL on failure.
 */
char *collatinus_lookup(const char *word, const char *lang);

/* Free a string returned by collatinus_lookup. */
void collatinus_free_result(char *result);

#ifdef __cplusplus
}
#endif
