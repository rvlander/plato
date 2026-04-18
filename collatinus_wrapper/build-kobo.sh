#! /bin/sh

# Cross-compile for Kobo (ARM).
# Qt headers are architecture-independent — use the host Qt5 installation.
# The host moc tool generates plain C++ that is then compiled by the ARM compiler.
GCC15=/opt/homebrew/Cellar/arm-unknown-linux-gnueabihf/15.2.0/bin
QT5_PREFIX=$(brew --prefix qt@5 2>/dev/null)
TARGET_OS=Kobo \
    CXX=${GCC15}/arm-linux-gnueabihf-g++ \
    AR=${GCC15}/arm-linux-gnueabihf-ar \
    QT5_PREFIX=${QT5_PREFIX} \
    BUILD_SHARED=1 \
    ./build.sh
