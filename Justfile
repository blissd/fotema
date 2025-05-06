# SPDX-FileCopyrightText: © 2024 David Bliss
#
# SPDX-License-Identifier: GPL-3.0-or-later

[private]
default:
    just --list --justfile {{ justfile() }}

# Cargo format
fmt:
  cargo fmt --verbose --all -- --check

# Cargo clippy
clippy:
  cargo clippy --all-targets --all-features -- -D warnings

# Run linters, such as the licence linter
lint:
    reuse lint
    flatpak run --command=flatpak-builder-lint org.flatpak.Builder manifest build-aux/app.fotema.Fotema.Devel.json

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

# Build and install flatpak development version
devel:
    flatpak run org.flatpak.Builder --user --install --force-clean _flatpak_app/devel build-aux/app.fotema.Fotema.Devel.json


# Created a vendors package that will be used by the flatpak-builder build for flathub.
# Use a separate _build_flathub directory because the meson version used by GNOME Builder
# clashes with the meson version installed natively.
dist:
    rm -rf _build_flathub
    meson setup _build_flathub
    meson dist -C _build_flathub

# Install Fedora development dependencies
setup:
    uv tool install --force reuse
    sudo dnf install -y libavformat-free-devel
    sudo dnf install -y libavfilter-free-devel
    sudo dnf install -y libavdevice-free-devel
    sudo dnf install -y clang-libs
    sudo dnf install -y clang-devel
    sudo dnf install -y libadwaita-devel
    sudo dnf install -y libshumate-devel
    sudo dnf install -y openssl-devel
    sudo dnf install -y opencv-core
    sudo dnf install -y vcpkg
