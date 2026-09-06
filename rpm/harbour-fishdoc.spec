Name:       harbour-fishdoc
Summary:    Notes++ — AsciiDoc notes app for Sailfish OS
Version:    0.1.0
Release:    30
Group:      Utilities
License:    MIT
URL:        https://github.com/gobuki/harbour-fishdoc
Source0:    %{name}-%{version}.tar.bz2
Requires:   sailfishsilica-qt5 >= 0.10.9
BuildRequires:  pkgconfig(sailfishapp) >= 1.0.2
BuildRequires:  pkgconfig(Qt5Core)
BuildRequires:  pkgconfig(Qt5Qml)
BuildRequires:  pkgconfig(Qt5Quick)
BuildRequires:  cargo
BuildRequires:  rust
BuildRequires:  rust-std-static
BuildRequires:  gcc-c++
BuildRequires:  meego-rpm-config

%description
Notes++ is an AsciiDoc notes app for Sailfish OS.
Journal with auto-managed daily named pages, page linking,
full-text search, HTML5 export, and embedded documentation web server.

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

rustc --version
cargo --version

cargo build --release --locked -p harbour-fishdoc

%install
rm -rf %{buildroot}
mkdir -p %{buildroot}%{_bindir}

# Find the binary
BINARY=$(find target -name harbour-fishdoc -type f -path "*/release/harbour-fishdoc" | head -1)
if [ -z "$BINARY" ]; then
  echo "ERROR: harbour-fishdoc binary not found in target/"
  find target -name harbour-fishdoc -type f
  exit 1
fi
install -m 755 "$BINARY" %{buildroot}%{_bindir}/%{name}

mkdir -p %{buildroot}%{_datadir}/%{name}/qml
cp -r fishdoc-sailfish/qml/* %{buildroot}%{_datadir}/%{name}/qml/

mkdir -p %{buildroot}%{_datadir}/%{name}/examples
cp fishdoc-core/examples/*.adoc %{buildroot}%{_datadir}/%{name}/examples/
cp fishdoc-core/examples/*.yml %{buildroot}%{_datadir}/%{name}/examples/ 2>/dev/null || true
cp fishdoc-core/examples/*.png %{buildroot}%{_datadir}/%{name}/examples/ 2>/dev/null || true
cp fishdoc-core/examples/*.jpg %{buildroot}%{_datadir}/%{name}/examples/ 2>/dev/null || true
cp fishdoc-core/examples/*.svg %{buildroot}%{_datadir}/%{name}/examples/ 2>/dev/null || true
cp rpm/%{name}.png %{buildroot}%{_datadir}/%{name}/examples/icon.png 2>/dev/null || true
if [ -d fishdoc-core/examples/chronicles ]; then
    mkdir -p %{buildroot}%{_datadir}/%{name}/examples/chronicles
    cp fishdoc-core/examples/chronicles/* %{buildroot}%{_datadir}/%{name}/examples/chronicles/
fi

mkdir -p %{buildroot}%{_datadir}/applications
cat > %{buildroot}%{_datadir}/applications/%{name}.desktop << EOF
[Desktop Entry]
Type=Application
X-Nemo-Application-Type=silica-qt5
Icon=%{name}
Exec=%{name}
Name=Notes++

[X-Sailjail]
Permissions=UserDirs;Internet;RemovableMedia
OrganizationName=org.gobuki
ApplicationName=harbour-fishdoc
EOF

mkdir -p %{buildroot}%{_sysconfdir}/sailjail/permissions
install -m 644 %{_sourcedir}/%{name}.profile %{buildroot}%{_sysconfdir}/sailjail/permissions/

mkdir -p %{buildroot}%{_datadir}/icons/hicolor/86x86/apps
install -m 644 rpm/%{name}.png %{buildroot}%{_datadir}/icons/hicolor/86x86/apps/%{name}.png

%files
%defattr(-,root,root,-)
%{_bindir}/%{name}
%{_datadir}/%{name}/qml
%{_datadir}/%{name}/examples
%{_datadir}/applications/%{name}.desktop
%{_datadir}/icons/hicolor/86x86/apps/%{name}.png
%config %{_sysconfdir}/sailjail/permissions/%{name}.profile
