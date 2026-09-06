#!/bin/bash
# Deploy an RPM package to Sailfish OS device and trigger installation prompt/notification.
#
# Usage:
#   ./deploy-sailfish.sh [path-to-rpm] [target-host]
#
# Examples:
#   ./deploy-sailfish.sh
#   ./deploy-sailfish.sh rpms/harbour-fishdoc-0.1.0-26.aarch64.rpm
#   ./deploy-sailfish.sh rpms/harbour-fishdoc-0.1.0-26.aarch64.rpm phone-wifi

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

RPM_PATH="$1"
TARGET_HOST="$2"

# 1. Locate RPM package
if [ -z "$RPM_PATH" ] || [ ! -f "$RPM_PATH" ]; then
    if [ -n "$RPM_PATH" ] && [ ! -f "$RPM_PATH" ]; then
        # Argument was given but is not a file; check if it was intended as target host
        if [ -z "$TARGET_HOST" ]; then
            TARGET_HOST="$RPM_PATH"
            RPM_PATH=""
        fi
    fi
fi

if [ -z "$RPM_PATH" ]; then
    # Look for the latest RPM in rpms/ directory or current directory
    RPM_PATH="$(ls -t "$SCRIPT_DIR/rpms/"*.rpm 2>/dev/null | head -n 1 || true)"
    if [ -z "$RPM_PATH" ]; then
        RPM_PATH="$(ls -t "$SCRIPT_DIR/"*.rpm 2>/dev/null | head -n 1 || true)"
    fi
fi

if [ -z "$RPM_PATH" ] || [ ! -f "$RPM_PATH" ]; then
    echo "ERROR: No RPM package found to deploy."
    echo "Please build the project first with ./build-sailfish.sh or pass an RPM path."
    echo "Usage: $0 [path-to-rpm] [target-host]"
    exit 1
fi

RPM_FILE="$(basename "$RPM_PATH")"
APP_NAME="harbour-fishdoc"

# 2. Determine target host
if [ -z "$TARGET_HOST" ]; then
    if [ -n "$PHONE_HOST" ]; then
        TARGET_HOST="$PHONE_HOST"
    else
        # Auto-detect reachable host among standard Sailfish SSH aliases
        for candidate in phone-wifi phone-usb phone-bt 192.168.1.200 192.168.2.15 172.28.172.1; do
            if ssh -o ConnectTimeout=2 -o BatchMode=yes "$candidate" true 2>/dev/null; then
                TARGET_HOST="$candidate"
                break
            fi
        done
        TARGET_HOST="${TARGET_HOST:-phone-wifi}"
    fi
fi

echo "=== Deploying $RPM_FILE to $TARGET_HOST ==="

# 3. Verify remote SSH connectivity and detect user/home
echo "Checking connection to $TARGET_HOST..."
REMOTE_INFO="$(ssh -o ConnectTimeout=5 "$TARGET_HOST" 'id -u; echo "$HOME"' 2>/dev/null)" || {
    echo "ERROR: Failed to connect to $TARGET_HOST via SSH."
    echo "Please ensure the phone is connected, developer mode / SSH is active, and the host is reachable."
    exit 1
}

REMOTE_UID="$(echo "$REMOTE_INFO" | sed -n '1p')"
REMOTE_HOME="$(echo "$REMOTE_INFO" | sed -n '2p')"

# Validate REMOTE_UID is numeric and REMOTE_HOME is an absolute path
if ! echo "$REMOTE_UID" | grep -qE '^[0-9]+$'; then
    echo "ERROR: Unexpected REMOTE_UID '$REMOTE_UID' — expected a numeric UID."
    echo "Raw remote info: $REMOTE_INFO"
    exit 1
fi
if [ -z "$REMOTE_HOME" ] || [ "${REMOTE_HOME#/}" = "$REMOTE_HOME" ]; then
    echo "ERROR: Unexpected REMOTE_HOME '$REMOTE_HOME' — expected an absolute path starting with /."
    echo "Raw remote info: $REMOTE_INFO"
    exit 1
fi
REMOTE_DOWNLOADS="$REMOTE_HOME/Downloads"
REMOTE_DEST="$REMOTE_DOWNLOADS/$RPM_FILE"

# 4. Copy RPM to phone's Downloads directory
echo "Stopping any running harbour-fishdoc processes on $TARGET_HOST..."
ssh "$TARGET_HOST" 'killall -9 harbour-fishdoc 2>/dev/null || true'

echo "Copying $RPM_FILE to $TARGET_HOST:$REMOTE_DOWNLOADS/..."
ssh "$TARGET_HOST" "mkdir -p '$REMOTE_DOWNLOADS'"
scp -p "$RPM_PATH" "$TARGET_HOST:$REMOTE_DEST"

# 5. Trigger installation dialog and notification on Sailfish OS via D-Bus
echo "Triggering package install dialog and notification on device..."
ssh "$TARGET_HOST" bash -s <<EOF
set -e
BUS="unix:path=/run/user/$REMOTE_UID/dbus/user_bus_socket"
if [ ! -e "/run/user/$REMOTE_UID/dbus/user_bus_socket" ]; then
    BUS="\$DBUS_SESSION_BUS_ADDRESS"
fi

# 1. Open package install dialog via Sailfish OS file service
DBUS_SESSION_BUS_ADDRESS="\$BUS" gdbus call --session \
    --dest org.sailfishos.fileservice \
    --object-path / \
    --method org.sailfishos.fileservice.openUrl \
    "file://$REMOTE_DEST" >/dev/null 2>&1 || true

# 2. Show system notification with install action
DBUS_SESSION_BUS_ADDRESS="\$BUS" dbus-send --session \
    --dest=org.freedesktop.Notifications \
    --type=method_call \
    /org/freedesktop/Notifications \
    org.freedesktop.Notifications.Notify \
    string:"$APP_NAME" \
    uint32:0 \
    string:"icon-m-service-download" \
    string:"$APP_NAME Update" \
    string:"Tap to install $RPM_FILE" \
    array:string:"default","Install" \
    dict:string:variant:"x-nemo-remote-action-default",string:"org.sailfishos.fileservice / org.sailfishos.fileservice openUrl file://$REMOTE_DEST","x-nemo-preview-summary",string:"$APP_NAME Update Ready","x-nemo-preview-body",string:"Tap to install $RPM_FILE" \
    int32:-1 >/dev/null 2>&1 || true
EOF

echo "=== Deployment complete! Check your phone to confirm installation. ==="
