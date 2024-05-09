# SPDX-FileCopyrightText: © 2024 David Bliss
#
# SPDX-License-Identifier: GPL-3.0-or-later

[private]
default:
    just --list --justfile {{ justfile() }}

# Run linters, such as the licence linter
lint:
    reuse lint

# Add licence information to all supported files
license:
    reuse annotate \
        --recursive \
        --skip-unrecognised \
        --skip-existing \
        --copyright "David Bliss" \
        --license "GPL-3.0-or-later" \
        --year `date +%Y` \
        --copyright-style spdx-symbol \
        .

# Build and install a flatpak release
release:
    flatpak-builder --user --install --force-clean flatpak_app build-aux/app.fotema.Fotema.json

devel:
    flatpak-builder --user --install --force-clean flatpak_app build-aux/app.fotema.Fotema.Devel.json

# Install Fedora development dependencies
setup:
    sudo dnf install -y libavformat-free-devel
    sudo dnf install -y libavfilter-free-devel
    sudo dnf install -y libavdevice-free-devel
    sudo dnf install -y clang-libs
    sudo dnf install -y clang-devel
    sudo dnf install -y libadwaita-devel
