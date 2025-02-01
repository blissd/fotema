#!/bin/sh
# SPDX-FileCopyrightText: © 2024 David Bliss
#
# SPDX-License-Identifier: GPL-3.0-or-later
#
# Since Meson invokes this script as
# "/bin/sh .../dist-vendor.sh DIST SOURCE_ROOT" we can't rely on bash features
set -eu
export DIST="$1"
export SOURCE_ROOT="$2"

cd "$SOURCE_ROOT"
mkdir "$DIST"/.cargo
cargo vendor > "$DIST/.cargo/config"
# Don't combine the previous and this line with a pipe because we can't catch
# errors with "set -o pipefail"
sed -i 's/^directory = ".*"/directory = "vendor"/g' "$DIST/.cargo/config"
# Move vendor into dist tarball directory
mv vendor "$DIST"
