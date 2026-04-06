#! /bin/sh

set -e

CXX=${CXX:-g++}
AR=${AR:-ar}

TARGET_OS=${TARGET_OS:-$(uname -s)}
BUILD_DIR=../target/collatinus_wrapper/${TARGET_OS}
mkdir -p ${BUILD_DIR}

# ---------------------------------------------------------------------------
# Locate Qt5 (or Qt6) include / link flags and MOC binary.
# The Collatinus library uses Q_OBJECT pervasively; MOC-generated sources
# must be compiled and linked to satisfy the vtable symbols.
# ---------------------------------------------------------------------------

# On macOS, Homebrew installs Qt5 at a keg-only path not on $PATH.
# Detect it explicitly before falling back to generic qmake.
# QT5_PREFIX can be passed in from the environment (e.g. by build-kobo.sh
# for cross-compilation, where the host Qt headers are used with the ARM compiler).
if [ -z "${QT5_PREFIX}" ] && [ "${TARGET_OS}" = "Darwin" ] && command -v brew >/dev/null 2>&1; then
    QT5_PREFIX=$(brew --prefix qt@5 2>/dev/null || true)
fi

if [ -n "${QT5_PREFIX}" ] && [ -x "${QT5_PREFIX}/bin/qmake" ]; then
    QMAKE="${QT5_PREFIX}/bin/qmake"
    MOC="${QT5_PREFIX}/bin/moc"
    QT_INSTALL_HEADERS=$(${QMAKE} -query QT_INSTALL_HEADERS)
    QT_INSTALL_LIBS=$(${QMAKE} -query QT_INSTALL_LIBS)
    QT_VERSION=$(${QMAKE} -query QT_VERSION | cut -d. -f1)
    QT_CXXFLAGS="-I${QT_INSTALL_HEADERS} -I${QT_INSTALL_HEADERS}/QtCore -DQT_CORE_LIB -DQT_NO_DEBUG"
    if [ "${TARGET_OS}" = "Darwin" ]; then
        QT_LIBS="-F${QT_INSTALL_LIBS} -framework QtCore"
    else
        QT_LIBS="-L${QT_INSTALL_LIBS} -lQt${QT_VERSION}Core"
    fi
elif pkg-config --exists Qt5Core 2>/dev/null; then
    QT_CXXFLAGS="$(pkg-config --cflags Qt5Core) -DQT_NO_DEBUG"
    QT_LIBS=$(pkg-config --libs Qt5Core)
    # Try to find moc next to pkg-config-reported include
    QT_INSTALL_HEADERS=$(pkg-config --variable=includedir Qt5Core)
    QT_INSTALL_LIBS=$(pkg-config --variable=libdir Qt5Core)
    # moc is usually in the bin next to the lib
    MOC=$(dirname $(pkg-config --variable=exec_prefix Qt5Core 2>/dev/null || echo ""))/bin/moc
    if [ ! -x "${MOC}" ]; then
        MOC=$(command -v moc || command -v moc-qt5 || echo "moc")
    fi
elif pkg-config --exists Qt6Core 2>/dev/null; then
    QT_CXXFLAGS="$(pkg-config --cflags Qt6Core) -DQT_NO_DEBUG"
    QT_LIBS=$(pkg-config --libs Qt6Core)
    QT_INSTALL_HEADERS=$(pkg-config --variable=includedir Qt6Core)
    MOC=$(command -v moc || command -v moc-qt6 || echo "moc")
elif command -v qmake >/dev/null 2>&1; then
    QMAKE=qmake
    MOC=moc
    QT_INSTALL_HEADERS=$(${QMAKE} -query QT_INSTALL_HEADERS)
    QT_INSTALL_LIBS=$(${QMAKE} -query QT_INSTALL_LIBS)
    QT_VERSION=$(${QMAKE} -query QT_VERSION | cut -d. -f1)
    QT_CXXFLAGS="-I${QT_INSTALL_HEADERS} -I${QT_INSTALL_HEADERS}/QtCore -DQT_CORE_LIB -DQT_NO_DEBUG"
    QT_LIBS="-L${QT_INSTALL_LIBS} -lQt${QT_VERSION}Core"
elif command -v qmake6 >/dev/null 2>&1; then
    QMAKE=qmake6
    MOC=moc
    QT_INSTALL_HEADERS=$(${QMAKE} -query QT_INSTALL_HEADERS)
    QT_INSTALL_LIBS=$(${QMAKE} -query QT_INSTALL_LIBS)
    QT_CXXFLAGS="-I${QT_INSTALL_HEADERS} -I${QT_INSTALL_HEADERS}/QtCore -DQT_CORE_LIB -DQT_NO_DEBUG"
    QT_LIBS="-L${QT_INSTALL_LIBS} -lQt6Core"
else
    echo "ERROR: Qt not found. Install Qt5 or Qt6 development packages." >&2
    echo "  On Debian/Ubuntu: apt install qtbase5-dev" >&2
    echo "  On Fedora/RHEL:   dnf install qt5-qtbase-devel" >&2
    echo "  On macOS (brew):  brew install qt@5" >&2
    exit 1
fi

COLLATINUS_SRC=../thirdparty/collatinus/src

# ---------------------------------------------------------------------------
# Compile Collatinus core source files (all files needed for lemmatization,
# excluding the Qt-network server, the Qt-XML dicos module, and the main
# entry point).
# ---------------------------------------------------------------------------
for src in \
    ${COLLATINUS_SRC}/ch.cpp \
    ${COLLATINUS_SRC}/irregs.cpp \
    ${COLLATINUS_SRC}/lemme.cpp \
    ${COLLATINUS_SRC}/modele.cpp \
    ${COLLATINUS_SRC}/lemCore.cpp \
    ${COLLATINUS_SRC}/lemmatiseur.cpp \
; do
    obj="${BUILD_DIR}/$(basename ${src} .cpp).o"
    ${CXX} ${CPPFLAGS} ${CXXFLAGS} ${QT_CXXFLAGS} \
        -std=c++14 \
        -DMEDIEVAL \
        -I${COLLATINUS_SRC} \
        -c "${src}" -o "${obj}"
done

# ---------------------------------------------------------------------------
# Run Qt MOC on all Collatinus headers that declare Q_OBJECT.
# The generated moc_*.cpp files define the vtables for signals/slots and
# staticMetaObject — without them the final link will fail with undefined
# symbols.
# ---------------------------------------------------------------------------
for hdr in lemCore.h lemmatiseur.h lemme.h modele.h irregs.h; do
    moc_cpp="${BUILD_DIR}/moc_${hdr%.h}.cpp"
    moc_obj="${BUILD_DIR}/moc_${hdr%.h}.o"
    ${MOC} \
        -I${QT_INSTALL_HEADERS} \
        -I${QT_INSTALL_HEADERS}/QtCore \
        -I${COLLATINUS_SRC} \
        -DMEDIEVAL \
        -DQT_CORE_LIB -DQT_NO_DEBUG \
        ${COLLATINUS_SRC}/${hdr} -o ${moc_cpp}
    ${CXX} ${CPPFLAGS} ${CXXFLAGS} ${QT_CXXFLAGS} \
        -std=c++14 \
        -DMEDIEVAL \
        -I${COLLATINUS_SRC} \
        -c "${moc_cpp}" -o "${moc_obj}"
done

# ---------------------------------------------------------------------------
# Compile the wrapper itself
# ---------------------------------------------------------------------------
${CXX} ${CPPFLAGS} ${CXXFLAGS} ${QT_CXXFLAGS} \
    -std=c++14 \
    -DMEDIEVAL \
    -I${COLLATINUS_SRC} \
    -c collatinus_wrapper.cpp -o ${BUILD_DIR}/collatinus_wrapper.o

ALL_OBJS="\
    ${BUILD_DIR}/collatinus_wrapper.o \
    ${BUILD_DIR}/ch.o \
    ${BUILD_DIR}/irregs.o \
    ${BUILD_DIR}/lemme.o \
    ${BUILD_DIR}/modele.o \
    ${BUILD_DIR}/lemCore.o \
    ${BUILD_DIR}/lemmatiseur.o \
    ${BUILD_DIR}/moc_lemCore.o \
    ${BUILD_DIR}/moc_lemmatiseur.o \
    ${BUILD_DIR}/moc_lemme.o \
    ${BUILD_DIR}/moc_modele.o \
    ${BUILD_DIR}/moc_irregs.o"

if [ "${BUILD_SHARED:-0}" = "1" ]; then
    # ---------------------------------------------------------------------------
    # Shared library for Kobo: Qt5Core is NOT linked here; it is loaded at
    # runtime via collatinus_preload() using dlopen(RTLD_GLOBAL).
    # --allow-shlib-undefined lets the link succeed with Qt symbols unresolved.
    # ---------------------------------------------------------------------------
    ${CXX} -shared \
        -Wl,--allow-shlib-undefined \
        -Wl,-soname,libcollatinus_wrapper.so \
        ${ALL_OBJS} \
        -ldl \
        -o ${BUILD_DIR}/libcollatinus_wrapper.so
    echo "Built: ${BUILD_DIR}/libcollatinus_wrapper.so"
else
    # ---------------------------------------------------------------------------
    # Static library (macOS / Linux host build — Qt5Core linked by the host)
    # ---------------------------------------------------------------------------
    ${AR} -rcs ${BUILD_DIR}/libcollatinus_wrapper.a ${ALL_OBJS}
    echo "Built: ${BUILD_DIR}/libcollatinus_wrapper.a"
    echo "Qt link flags (needed by the final binary): ${QT_LIBS}"
fi
