#!/bin/sh
# Post-install steps for the Murmur Debian package.
set -e

case "$1" in
    configure|reconfigure|"")
        # Rename the Tauri-generated .desktop file so it matches the
        # application id used by xdg-desktop-portal (kept around for any
        # portal interactions Murmur may still make for non-shortcut APIs).
        OLD=/usr/share/applications/Murmur.desktop
        NEW=/usr/share/applications/com.murmurlinux.murmur.desktop
        if [ -f "$OLD" ]; then
            mv -f "$OLD" "$NEW"
        fi
        if command -v update-desktop-database >/dev/null 2>&1; then
            update-desktop-database -q /usr/share/applications || true
        fi

        # Apply setgid `input` to the privileged keyboard helper so it can
        # open /dev/input/event* without the calling user being in the
        # input group. The helper drops gid back to the caller's real gid
        # immediately after opening the devices. Same shape as
        # /usr/bin/dumpcap shipped by the wireshark-common package.
        HELPER=/usr/bin/murmur-input-helper
        if [ -x "$HELPER" ]; then
            if getent group input >/dev/null; then
                chown root:input "$HELPER"
                chmod 02755 "$HELPER"
            else
                echo "murmur: 'input' group not found; helper will not be privileged." >&2
                chmod 0755 "$HELPER"
            fi
        fi

        # Reload udev so the rules we ship in
        # /lib/udev/rules.d/99-murmur.rules take effect. `event*`
        # devices live under the `input` subsystem; `uinput` lives
        # under `misc`. Trigger both.
        if command -v udevadm >/dev/null 2>&1; then
            udevadm control --reload-rules || true
            udevadm trigger --subsystem-match=input || true
            udevadm trigger --subsystem-match=misc || true
        fi

        # The /dev/uinput node was created by the kernel at module
        # load time, BEFORE our udev rule was on the system, so its
        # owner/mode are still the kernel defaults (root:root 0600).
        # `udevadm trigger` does not retroactively re-apply
        # GROUP=/MODE= clauses to existing nodes; only the
        # OPTIONS+="static_node=uinput" line in our rule fixes that,
        # and only at next boot. Apply the same perms directly here so
        # the helper can use uinput in the current session too.
        if [ -e /dev/uinput ] && getent group input >/dev/null; then
            chgrp input /dev/uinput || true
            chmod 0660 /dev/uinput || true
        fi
        ;;

    abort-upgrade|abort-remove|abort-deconfigure)
        ;;
esac

exit 0
