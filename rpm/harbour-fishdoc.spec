Name:       harbour-fishdoc
Summary:    FishDoc — AsciiDoc notes app for Sailfish OS
Version:    0.1.0
Release:    1
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
FishDoc is an AsciiDoc notes app for Sailfish OS.
Journal with auto-managed daily named pages, page linking,
full-text search, and PDF export.

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

cargo build --release -p harbour-fishdoc

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

mkdir -p %{buildroot}%{_datadir}/applications
cat > %{buildroot}%{_datadir}/applications/%{name}.desktop << EOF
[Desktop Entry]
Type=Application
X-Nemo-Application-Type=silica-qt5
Icon=%{name}
Exec=%{name}
Name=FishDoc

[X-Sailjail]
Permissions=UserDirs
PrivateBin=harbour-fishdoc
OrganizationName=org.gobuki
ApplicationName=harbour-fishdoc
EOF

mkdir -p %{buildroot}%{_sysconfdir}/sailjail/permissions
install -m 644 %{_sourcedir}/%{name}.profile %{buildroot}%{_sysconfdir}/sailjail/permissions/

mkdir -p %{buildroot}%{_datadir}/icons/hicolor/86x86/apps
install -m 644 %{_sourcedir}/%{name}.png %{buildroot}%{_datadir}/icons/hicolor/86x86/apps/%{name}.png 2>/dev/null || true

%files
%defattr(-,root,root,-)
%{_bindir}/%{name}
%{_datadir}/%{name}/qml
%{_datadir}/%{name}/examples
%{_datadir}/applications/%{name}.desktop
%config %{_sysconfdir}/sailjail/permissions/%{name}.profile
