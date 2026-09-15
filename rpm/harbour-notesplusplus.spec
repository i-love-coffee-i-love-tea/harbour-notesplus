Name:       harbour-notesplusplus
Summary:    Notes++ — AsciiDoc notes app for Sailfish OS
Version:    0.1.0
Release:    1
Group:      Utilities
License:    MIT
URL:        https://github.com/gobuki/harbour-notesplusplus
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
Notes++ is an AsciiDoc notes app for Sailfish OS.
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
cargo build --release --locked -p notesplusplus-core -j 1

# Locate the static library (cross-compilation puts it in target/<triple>/release/)
RUST_TARGET_DIR="target/${SB2_RUST_TARGET_TRIPLE}/release"
if [ ! -f "$RUST_TARGET_DIR/libnotesplusplus_core.a" ]; then
    RUST_TARGET_DIR="target/release"
fi
ls -la "$RUST_TARGET_DIR/libnotesplusplus_core.a" || {
  echo "ERROR: libnotesplusplus_core.a not found after cargo build"
  exit 1
}

# Step 2: Build the C++ bridge using qmake (links the Rust static lib)
cd notesplusplus-sailfish
qmake harbour-notesplusplus.pro \
    "RUST_CORE_LIB=$PWD/../$RUST_TARGET_DIR/libnotesplusplus_core.a"
make -j$(nproc)
cd ..

# Compile translations
cd notesplusplus-sailfish/translations
lrelease harbour-notesplusplus.ts harbour-notesplusplus_de.ts harbour-notesplusplus_es.ts harbour-notesplusplus_en.ts harbour-notesplusplus_en_US.ts
cd ../..

%install
rm -rf %{buildroot}
mkdir -p %{buildroot}%{_bindir}

# Find the binary (built by qmake in notesplusplus-sailfish/)
BINARY=""
for candidate in \
    notesplusplus-sailfish/harbour-notesplusplus \
    notesplusplus-sailfish/release/harbour-notesplusplus \
    target/release/harbour-notesplusplus; do
  if [ -f "$candidate" ] && [ -x "$candidate" ]; then
    BINARY="$candidate"
    break
  fi
done
if [ -z "$BINARY" ]; then
  echo "ERROR: harbour-notesplusplus binary not found"
  find . -name harbour-notesplusplus -type f -executable
  exit 1
fi
install -m 755 "$BINARY" %{buildroot}%{_bindir}/%{name}

mkdir -p %{buildroot}%{_datadir}/%{name}/qml
cp -r notesplusplus-sailfish/qml/* %{buildroot}%{_datadir}/%{name}/qml/

mkdir -p %{buildroot}%{_datadir}/%{name}/examples
cp -r notesplusplus-core/examples/* %{buildroot}%{_datadir}/%{name}/examples/
cp rpm/%{name}.png %{buildroot}%{_datadir}/%{name}/examples/icon.png 2>/dev/null || true

mkdir -p %{buildroot}%{_datadir}/applications
install -m 644 rpm/%{name}.desktop %{buildroot}%{_datadir}/applications/

for SIZE in 86 108 128 172; do
  mkdir -p %{buildroot}%{_datadir}/icons/hicolor/${SIZE}x${SIZE}/apps
  install -m 644 rpm/icons/${SIZE}x${SIZE}/%{name}.png %{buildroot}%{_datadir}/icons/hicolor/${SIZE}x${SIZE}/apps/%{name}.png
done

# Install translations
mkdir -p %{buildroot}%{_datadir}/%{name}/translations
install -m 644 notesplusplus-sailfish/translations/*.qm %{buildroot}%{_datadir}/%{name}/translations/

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
