#!/bin/sh
# Pre-remove steps for the Murmur Debian package.
#
# Clear the setgid bit on the helper before dpkg unlinks it, so the
# package manager does not stumble on the permission-modified file.
set -e

case "$1" in
    remove|upgrade|deconfigure|"")
        HELPER=/usr/libexec/murmur/murmur-input-helper
        if [ -e "$HELPER" ]; then
            chmod 0755 "$HELPER" || true
        fi
        ;;

    failed-upgrade)
        ;;
esac

exit 0
