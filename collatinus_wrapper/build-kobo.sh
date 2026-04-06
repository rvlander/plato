#! /bin/sh

# Cross-compile for Kobo (ARM).
# Qt headers are architecture-independent — use the host Qt5 installation.
# The host moc tool generates plain C++ that is then compiled by the ARM compiler.
QT5_PREFIX=$(brew --prefix qt@5 2>/dev/null)
TARGET_OS=Kobo \
    CXX=arm-linux-gnueabihf-g++ \
    AR=arm-linux-gnueabihf-ar \
    QT5_PREFIX=${QT5_PREFIX} \
    BUILD_SHARED=1 \
    ./build.sh
