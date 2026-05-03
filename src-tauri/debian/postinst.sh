#!/bin/sh
# Post-install steps for the Murmur Debian package.
set -e

# 1. Rename the Tauri-generated .desktop file so it matches the
#    application id used by xdg-desktop-portal (kept around for any
#    portal interactions Murmur may still make for non-shortcut APIs).
OLD=/usr/share/applications/Murmur.desktop
NEW=/usr/share/applications/com.murmurlinux.murmur.desktop
if [ -f "$OLD" ]; then
    mv -f "$OLD" "$NEW"
fi
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database -q /usr/share/applications || true
fi

# 2. Reload udev so the input-device ACL rule we ship in
#    /lib/udev/rules.d/99-murmur.rules takes effect on the user's next
#    login (and on currently-attached devices via udevadm trigger).
if command -v udevadm >/dev/null 2>&1; then
    udevadm control --reload-rules || true
    udevadm trigger --subsystem-match=input || true
fi
