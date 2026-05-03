#!/bin/sh
# Rename the Tauri-generated .desktop file so it matches the application id
# used by xdg-desktop-portal. The portal looks up `<app_id>.desktop` in the
# standard XDG application paths to authorise GlobalShortcuts requests; a
# missing or differently-named file causes the consent dialog to silently
# never appear.
set -e

OLD=/usr/share/applications/Murmur.desktop
NEW=/usr/share/applications/com.murmurlinux.murmur.desktop

if [ -f "$OLD" ]; then
    mv -f "$OLD" "$NEW"
fi

if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database -q /usr/share/applications || true
fi
