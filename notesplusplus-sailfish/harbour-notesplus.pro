# harbour-notesplus.pro — qmake project for Harbour-compliant build
#
# Builds the C++ bridge layer + links the Rust core as a static library.
# The Rust core is compiled via cargo (invoked from %build in the spec file).

TEMPLATE = app
TARGET = harbour-notesplus
CONFIG += sailfishapp link_pkgconfig
QT += quick qml network multimedia

# C++ standard
CONFIG += c++17
QMAKE_CXXFLAGS += -std=c++17

# Source files
SOURCES += \
    src/main.cpp \
    src/bridge/NotesBridge.cpp \
    src/bridge/AgentBridge.cpp \
    src/bridge/SpeechBridge.cpp

HEADERS += \
    src/ffi/notesplusplus_core.h \
    src/ffi/ffi_raii.h \
    src/bridge/NotesBridge.h \
    src/bridge/AgentBridge.h \
    src/bridge/SpeechBridge.h

# Include path for FFI headers
INCLUDEPATH += src/ffi

# Link the Rust static library
# The .a is built by cargo in the spec file's %build section
# RUST_CORE_LIB can be passed from the spec: qmake "RUST_CORE_LIB=/path/to/lib.a"
isEmpty(RUST_CORE_LIB) {
    RUST_CORE_LIB = $$OUT_PWD/../target/release/libnotesplusplus_core.a
}
!exists($$RUST_CORE_LIB) {
    # Cross-compilation target (sfdk)
    RUST_CORE_LIB = $$PWD/../target/aarch64-unknown-linux-gnu/release/libnotesplusplus_core.a
}
!exists($$RUST_CORE_LIB) {
    RUST_CORE_LIB = $$PWD/../target/release/libnotesplusplus_core.a
}
exists($$RUST_CORE_LIB) {
    PRE_TARGETDEPS += $$RUST_CORE_LIB
    LIBS += $$RUST_CORE_LIB
} else {
    error("Cannot find libnotesplusplus_core.a — build notesplusplus-core first with: cargo build --release -p notesplusplus-core")
}

# System libraries required by the Rust static lib
LIBS += -lpthread -lm -ldl -lsqlite3

# pkg-config packages
PKGCONFIG += sailfishapp sqlite3

# Install paths (handled by sailfishapp feature)
# Binary: /usr/bin/harbour-notesplus
# QML:   /usr/share/harbour-notesplus/qml/
