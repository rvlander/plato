#! /bin/sh

set -e

CXX=${CXX:-g++}
AR=${AR:-ar}

TARGET_OS=${TARGET_OS:-$(uname -s)}
BUILD_DIR=../target/collatinus_wrapper/${TARGET_OS}
mkdir -p ${BUILD_DIR}

# ---------------------------------------------------------------------------
# Locate Qt5 (or Qt6) include / link flags via pkg-config or qmake.
# The library is Qt-based and cannot be compiled without Qt headers.
# ---------------------------------------------------------------------------
if pkg-config --exists Qt5Core 2>/dev/null; then
    QT_CXXFLAGS=$(pkg-config --cflags Qt5Core)
    QT_LIBS=$(pkg-config --libs Qt5Core)
elif pkg-config --exists Qt6Core 2>/dev/null; then
    QT_CXXFLAGS=$(pkg-config --cflags Qt6Core)
    QT_LIBS=$(pkg-config --libs Qt6Core)
elif command -v qmake >/dev/null 2>&1; then
    QT_INSTALL_HEADERS=$(qmake -query QT_INSTALL_HEADERS)
    QT_INSTALL_LIBS=$(qmake -query QT_INSTALL_LIBS)
    QT_VERSION=$(qmake -query QT_VERSION | cut -d. -f1)
    QT_CXXFLAGS="-I${QT_INSTALL_HEADERS} -I${QT_INSTALL_HEADERS}/QtCore"
    QT_LIBS="-L${QT_INSTALL_LIBS} -lQt${QT_VERSION}Core"
elif command -v qmake6 >/dev/null 2>&1; then
    QT_INSTALL_HEADERS=$(qmake6 -query QT_INSTALL_HEADERS)
    QT_INSTALL_LIBS=$(qmake6 -query QT_INSTALL_LIBS)
    QT_CXXFLAGS="-I${QT_INSTALL_HEADERS} -I${QT_INSTALL_HEADERS}/QtCore"
    QT_LIBS="-L${QT_INSTALL_LIBS} -lQt6Core"
else
    echo "ERROR: Qt not found. Install Qt5 or Qt6 development packages." >&2
    echo "  On Debian/Ubuntu: apt install qtbase5-dev" >&2
    echo "  On Fedora/RHEL:   dnf install qt5-qtbase-devel" >&2
    echo "  On macOS (brew):  brew install qt" >&2
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
        -std=c++17 \
        -DMEDIEVAL \
        -I${COLLATINUS_SRC} \
        -c "${src}" -o "${obj}"
done

# ---------------------------------------------------------------------------
# Compile the wrapper itself
# ---------------------------------------------------------------------------
${CXX} ${CPPFLAGS} ${CXXFLAGS} ${QT_CXXFLAGS} \
    -std=c++17 \
    -DMEDIEVAL \
    -I${COLLATINUS_SRC} \
    -c collatinus_wrapper.cpp -o ${BUILD_DIR}/collatinus_wrapper.o

# ---------------------------------------------------------------------------
# Archive into a static library
# ---------------------------------------------------------------------------
${AR} -rcs ${BUILD_DIR}/libcollatinus_wrapper.a \
    ${BUILD_DIR}/collatinus_wrapper.o \
    ${BUILD_DIR}/ch.o \
    ${BUILD_DIR}/irregs.o \
    ${BUILD_DIR}/lemme.o \
    ${BUILD_DIR}/modele.o \
    ${BUILD_DIR}/lemCore.o \
    ${BUILD_DIR}/lemmatiseur.o

echo "Built: ${BUILD_DIR}/libcollatinus_wrapper.a"
echo "Qt link flags (needed by the final binary): ${QT_LIBS}"
