#!/bin/sh
# Wraps `tauri` to merge the dev-only config (withGlobalTauri, mcp-bridge
# capability) into `tauri dev`, keeping the base tauri.conf.json release-safe.
set -e

if [ "$1" = "dev" ]; then
    shift
    exec tauri dev --config src-tauri/tauri.dev.conf.json "$@"
else
    exec tauri "$@"
fi
