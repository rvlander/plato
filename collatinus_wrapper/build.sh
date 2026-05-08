#! /bin/sh

set -e

CXX=${CXX:-g++}
AR=${AR:-ar}

TARGET_OS=${TARGET_OS:-$(uname -s)}
BUILD_DIR=../target/collatinus_wrapper/${TARGET_OS}
mkdir -p ${BUILD_DIR}

COLLATINUS_DIR=${COLLATINUS_DIR:-../thirdparty/collatinus}
COLLATINUS_SRC=${COLLATINUS_DIR}/src

ALL_OBJS=""
for src in \
    ${COLLATINUS_SRC}/ch.cpp \
    ${COLLATINUS_SRC}/irregs.cpp \
    ${COLLATINUS_SRC}/lemme.cpp \
    ${COLLATINUS_SRC}/modele.cpp \
    ${COLLATINUS_SRC}/lemCore.cpp \
    ${COLLATINUS_SRC}/lemmatiseur.cpp \
    collatinus_wrapper.cpp \
; do
    obj="${BUILD_DIR}/$(basename ${src} .cpp).o"
    ${CXX} ${CPPFLAGS} ${CXXFLAGS} \
        -std=c++11 \
        -fPIC \
        -DMEDIEVAL \
        -I${COLLATINUS_SRC} \
        -c "${src}" -o "${obj}"
    ALL_OBJS="${ALL_OBJS} ${obj}"
done

${AR} -rcs ${BUILD_DIR}/libcollatinus_wrapper.a ${ALL_OBJS}

echo "Built: ${BUILD_DIR}/libcollatinus_wrapper.a"

echo "Built: ${BUILD_DIR}/libcollatinus_wrapper.so"
