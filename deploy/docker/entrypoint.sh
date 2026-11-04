#!/bin/sh
set -eu

# Summer still owns TOML loading; only prepare the fixed Docker layout here.
for file in /app/config/app.toml /app/config/app-prod.toml; do
    if [ ! -r "$file" ]; then
        echo "Missing readable Docker configuration: $file" >&2
        exit 1
    fi
    if grep -q 'REPLACE_WITH_' "$file"; then
        echo "Replace the example settings in $file before starting the app." >&2
        exit 1
    fi
done
for directory in /var/lib/viewer/jni /var/lib/viewer/pacs-viewer-core-downloads; do
    mkdir -p "$directory"
    probe=$(mktemp "$directory/.write-check.XXXXXX")
    rm "$probe"
done
exec /app/viewer-app "$@"
