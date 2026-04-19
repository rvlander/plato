#! /bin/sh

LINARO=/Users/rvlander/Downloads/gcc-linaro-4.9.4-2017.01-20170615/bin

TARGET_OS=Kobo \
    CXX=${LINARO}/arm-linux-gnueabihf-g++ \
    AR=${LINARO}/arm-linux-gnueabihf-ar \
    ./build.sh
