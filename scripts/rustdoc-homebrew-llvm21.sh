#!/bin/sh

export DYLD_LIBRARY_PATH="/opt/homebrew/Cellar/llvm/21.1.8/lib${DYLD_LIBRARY_PATH:+:$DYLD_LIBRARY_PATH}"
exec /opt/homebrew/bin/rustdoc "$@"
