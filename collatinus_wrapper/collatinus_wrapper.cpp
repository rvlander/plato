/*
 * collatinus_wrapper.cpp
 *
 * Thin C API wrapper around the Collatinus Latin morphological analysis library.
 *
 * The Collatinus library is Qt-based. This wrapper requires Qt5Core (or Qt6Core)
 * to be present at compile time and at runtime.
 *
 * A QCoreApplication is created lazily so that Qt's internal machinery works
 * even when called from a non-Qt host (e.g. a Rust binary).
 *
 * Data files are expected at the path set by COLLATINUS_DATA environment
 * variable, or next to the binary in a "data/" subdirectory as Collatinus
 * does by default.
 */

#include "collatinus_wrapper.h"

#include <cstdlib>
#include <cstring>
#include <mutex>
#include <memory>
#include <string>

#if defined(__linux__) && defined(__arm__)
#  include <dlfcn.h>
#endif

// Qt forward declarations needed before Collatinus headers
#include <QCoreApplication>
#include <QString>

#include "../thirdparty/collatinus/src/lemCore.h"
#include "../thirdparty/collatinus/src/lemmatiseur.h"

// ---------------------------------------------------------------------------
// Lazy singleton state
// ---------------------------------------------------------------------------

static std::unique_ptr<QCoreApplication> g_app;
static std::unique_ptr<LemCore>          g_lemCore;
static std::unique_ptr<Lemmatiseur>      g_lemmat;
static std::once_flag                    g_init_flag;

static void ensure_initialized() {
    std::call_once(g_init_flag, []() {
        // QCoreApplication is required by Qt's object system even in a
        // non-GUI context.  If one already exists (e.g. the host is itself a
        // Qt app) we must not create a second one.
        if (!QCoreApplication::instance()) {
            // We need a fake argc/argv that persists for the lifetime of the app.
            static int   fake_argc = 1;
            static char  fake_arg0[] = "collatinus_wrapper";
            static char* fake_argv[] = { fake_arg0, nullptr };
            g_app = std::make_unique<QCoreApplication>(fake_argc, fake_argv);
        }

        // Determine the data directory.
        // Priority: COLLATINUS_DATA env var, then <app_dir>/data/
        QString resDir;
        const char *env = std::getenv("COLLATINUS_DATA");
        if (env && *env) {
            resDir = QString::fromUtf8(env);
            if (!resDir.endsWith('/')) resDir += '/';
        }
        // If resDir is empty, LemCore will fall back to qApp->applicationDirPath()+"/data/"

        g_lemCore = std::make_unique<LemCore>(nullptr, resDir);
        g_lemCore->setExtension(true);

        g_lemmat = std::make_unique<Lemmatiseur>(nullptr, g_lemCore.get(), "", resDir);
        g_lemmat->setHtml(true);
        g_lemmat->setMorpho(true);
        // Show the word form at the head of each entry
        g_lemmat->setFormeT(true);
    });
}

// ---------------------------------------------------------------------------
// Public C API
// ---------------------------------------------------------------------------

extern "C" {

void collatinus_preload(void) {
#if defined(__linux__) && defined(__arm__)
    // Qt5Core is not linked at cross-compile time for Kobo. Load it now into
    // the global namespace so that the Qt symbols used by the Collatinus code
    // (which is in this shared library) are resolved before their first call.
    static const char * const paths[] = {
        "/usr/local/Qt-5.2.1-arm/lib/libQt5Core.so.5",
        "libQt5Core.so.5",
        "libQt5Core.so",
        nullptr
    };
    for (int i = 0; paths[i]; ++i) {
        if (dlopen(paths[i], RTLD_LAZY | RTLD_GLOBAL))
            return;
    }
#endif
}

char *collatinus_lookup(const char *word, const char *lang) {
    if (!word || !lang) return nullptr;
    try {
        ensure_initialized();

        // Switch output language for this call
        g_lemmat->setCible(QString::fromUtf8(lang));

        QString qword = QString::fromUtf8(word);
        // lemmatiseT takes the string by non-const reference (it may modify it
        // when colourising, but we don't use that feature).
        std::string result = g_lemmat->lemmatiseT(qword).toStdString();
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
