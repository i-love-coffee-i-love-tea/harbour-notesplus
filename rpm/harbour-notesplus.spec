Name:       harbour-notesplus
Summary:    Notes Plus — AsciiDoc notes app for Sailfish OS
Version:    0.3.0
Release:    1
Group:      Utilities
License:    MIT
URL:        https://github.com/gobuki/harbour-notesplus
Source0:    %{name}-%{version}.tar.bz2
Requires:   sailfishsilica-qt5
BuildRequires:  pkgconfig(sailfishapp) >= 1.0.2
BuildRequires:  pkgconfig(Qt5Core)
BuildRequires:  pkgconfig(Qt5Qml)
BuildRequires:  pkgconfig(Qt5Quick)
BuildRequires:  pkgconfig(Qt5Network)
BuildRequires:  pkgconfig(Qt5Multimedia)
BuildRequires:  pkgconfig(sqlite3)
BuildRequires:  cargo
BuildRequires:  rust
BuildRequires:  rust-std-static
BuildRequires:  gcc-c++
BuildRequires:  qt5-qttools-linguist

%description
Notes Plus is an AsciiDoc notes app for Sailfish OS.
Journal with auto-managed daily named pages, page linking,
full-text search, HTML5 export, and embedded documentation web server.

# >> macros
%define __provides_exclude_from ^%{_bindir}/.*$
# << macros

%prep
%setup -q -n %{name}-%{version}

%build

# qmake setup
export QMAKE=/usr/bin/qmake

# Cross-compilation setup
%ifarch aarch64
export SB2_RUST_TARGET_TRIPLE=aarch64-unknown-linux-gnu
export CFLAGS_aarch64_unknown_linux_gnu=$CFLAGS
export CXXFLAGS_aarch64_unknown_linux_gnu=$CXXFLAGS
%endif

%ifarch armv7hl
export SB2_RUST_TARGET_TRIPLE=armv7-unknown-linux-gnueabihf
export CFLAGS_armv7_unknown_linux_gnueabihf=$CFLAGS
export CXXFLAGS_armv7_unknown_linux_gnueabihf=$CXXFLAGS
%endif

%ifarch %ix86
export SB2_RUST_TARGET_TRIPLE=i686-unknown-linux-gnu
export CFLAGS_i686_unknown_linux_gnu=$CFLAGS
export CXXFLAGS_i686_unknown_linux_gnu=$CXXFLAGS
%endif

rustc --version
cargo --version

export CARGO_BUILD_JOBS=1
export RUSTFLAGS="-C link-arg=-Wl,--as-needed"

# Step 1: Build the Rust core as a static library (.a)
cargo build --release --locked -p notesplus-core -j 1

# Locate the static library (cross-compilation puts it in target/<triple>/release/)
RUST_TARGET_DIR="target/${SB2_RUST_TARGET_TRIPLE}/release"
if [ ! -f "$RUST_TARGET_DIR/libnotesplus_core.a" ]; then
    RUST_TARGET_DIR="target/release"
fi
ls -la "$RUST_TARGET_DIR/libnotesplus_core.a" || {
  echo "ERROR: libnotesplus_core.a not found after cargo build"
  exit 1
}

# Step 2: Build the C++ bridge using qmake (links the Rust static lib)
cd notesplus-sailfish
qmake harbour-notesplus.pro \
    "RUST_CORE_LIB=$PWD/../$RUST_TARGET_DIR/libnotesplus_core.a"
make -j$(nproc)
cd ..

# Compile translations
cd notesplus-sailfish/translations
lrelease harbour-notesplus.ts harbour-notesplus_de.ts harbour-notesplus_es.ts harbour-notesplus_en.ts harbour-notesplus_en_US.ts
cd ../..

%install
rm -rf %{buildroot}
mkdir -p %{buildroot}%{_bindir}

# Find the binary (built by qmake in notesplus-sailfish/)
BINARY=""
for candidate in \
    notesplus-sailfish/harbour-notesplus \
    notesplus-sailfish/release/harbour-notesplus \
    target/release/harbour-notesplus; do
  if [ -f "$candidate" ] && [ -x "$candidate" ]; then
    BINARY="$candidate"
    break
  fi
done
if [ -z "$BINARY" ]; then
  echo "ERROR: harbour-notesplus binary not found"
  find . -name harbour-notesplus -type f -executable
  exit 1
fi
install -m 755 "$BINARY" %{buildroot}%{_bindir}/%{name}

mkdir -p %{buildroot}%{_datadir}/%{name}/qml
cp -r notesplus-sailfish/qml/* %{buildroot}%{_datadir}/%{name}/qml/

mkdir -p %{buildroot}%{_datadir}/%{name}/examples
cp -r notesplus-core/examples/* %{buildroot}%{_datadir}/%{name}/examples/
mkdir -p %{buildroot}%{_datadir}/%{name}/examples/Notes_Plus_Documentation/ADRs
cp docs/dev/adr/*.adoc %{buildroot}%{_datadir}/%{name}/examples/Notes_Plus_Documentation/ADRs/
cp rpm/%{name}.png %{buildroot}%{_datadir}/%{name}/examples/icon.png 2>/dev/null || true

mkdir -p %{buildroot}%{_datadir}/applications
install -m 644 rpm/%{name}.desktop %{buildroot}%{_datadir}/applications/

for SIZE in 86 108 128 172; do
  mkdir -p %{buildroot}%{_datadir}/icons/hicolor/${SIZE}x${SIZE}/apps
  install -m 644 rpm/icons/${SIZE}x${SIZE}/%{name}.png %{buildroot}%{_datadir}/icons/hicolor/${SIZE}x${SIZE}/apps/%{name}.png
done

# Install translations
mkdir -p %{buildroot}%{_datadir}/%{name}/translations
install -m 644 notesplus-sailfish/translations/*.qm %{buildroot}%{_datadir}/%{name}/translations/

# Install AppStream metadata
mkdir -p %{buildroot}%{_datadir}/metainfo
install -m 644 rpm/%{name}.appdata.xml %{buildroot}%{_datadir}/metainfo/%{name}.metainfo.xml

%files
%defattr(-,root,root,-)
%{_bindir}/%{name}
%{_datadir}/%{name}/qml
%{_datadir}/%{name}/translations
%{_datadir}/%{name}/examples
%{_datadir}/applications/%{name}.desktop
%{_datadir}/icons/hicolor/86x86/apps/%{name}.png
%{_datadir}/icons/hicolor/108x108/apps/%{name}.png
%{_datadir}/icons/hicolor/128x128/apps/%{name}.png
%{_datadir}/icons/hicolor/172x172/apps/%{name}.png
%{_datadir}/metainfo/%{name}.metainfo.xml
