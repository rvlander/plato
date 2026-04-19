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

#ifdef __cplusplus
}
#endif
