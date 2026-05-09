/*
 * collatinus_wrapper.cpp
 *
 * Thin C API wrapper around the Collatinus Latin morphological analysis
 * library (no-qt fork — pure C++11 stdlib, no Qt dependency).
 *
 * Data files are expected at the path set by the COLLATINUS_DATA environment
 * variable, or next to the binary in a "data/" subdirectory.
 */

#include "collatinus_wrapper.h"

#include <cstdlib>
#include <cstring>
#include <mutex>
#include <memory>
#include <string>

#include "lemCore.h"
#include "lemmatiseur.h"

static std::unique_ptr<LemCore>     g_lemCore;
static std::unique_ptr<Lemmatiseur> g_lemmat;
static std::once_flag               g_init_flag;
static std::string                  g_lang = "fr";

static void ensure_initialized() {
    std::call_once(g_init_flag, []() {
        // Catch inside the callable: some libc++ versions leave once_flag
        // stuck in "in-progress" state if the callable throws, causing any
        // subsequent call_once on another thread to hang forever.
        try {
            std::string resDir;
            const char *env = std::getenv("COLLATINUS_DATA");
            if (env && *env) {
                resDir = env;
                if (resDir.back() != '/') resDir += '/';
            }

            g_lemCore.reset(new LemCore(resDir, g_lang));
            g_lemCore->setExtension(true);

            g_lemmat.reset(new Lemmatiseur(g_lemCore.get(), resDir));
            g_lemmat->setHtml(true);
            g_lemmat->setMorpho(true);
            g_lemmat->setFormeT(true);
        } catch (...) {}
    });
}

extern "C" {

int collatinus_init(const char *lang) {
    if (!lang || !*lang) return -1;
    g_lang = lang;
    ensure_initialized();
    return g_lemCore ? 0 : -1;
}

char *collatinus_lookup(const char *word, const char *lang) {
    if (!word || !lang) return nullptr;
    try {
        ensure_initialized();

        g_lemmat->setCible(std::string(lang));

        std::string w(word);
        std::string result = g_lemmat->lemmatiseT(w);
        if (result.empty()) return nullptr;

        char *out = static_cast<char *>(std::malloc(result.size() + 1));
        if (!out) return nullptr;
        std::memcpy(out, result.c_str(), result.size() + 1);
        return out;
    } catch (...) {
        return nullptr;
    }
}

void collatinus_free_result(char *result) {
    std::free(result);
}

} // extern "C"
