# SPDX-FileCopyrightText: © 2024 David Bliss
#
# SPDX-License-Identifier: GPL-3.0-or-later

[private]
default:
    just --list --justfile {{ justfile() }}

# Run linters, such as the licence linter
lint:
    reuse lint
    flatpak run --command=flatpak-builder-lint org.flatpak.Builder manifest build-aux/app.fotema.Fotema.Devel.json
    flatpak run --command=flatpak-builder-lint org.flatpak.Builder manifest build-aux/app.fotema.Fotema.json
    #flatpak run --command=flatpak-builder-lint org.flatpak.Builder appstream flatpak_app/files/share/metainfo/app.fotema.Fotema.Devel.metainfo.xml
    #flatpak run --command=flatpak-builder-lint org.flatpak.Builder repo fotema-origin

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
    flatpak-builder --user --install --force-clean flatpak_app/release build-aux/app.fotema.Fotema.json

devel:
    flatpak-builder --user --install --force-clean flatpak_app/devel build-aux/app.fotema.Fotema.Devel.json

flathub:
    flatpak run org.flatpak.Builder --force-clean --sandbox --user --install --ccache --mirror-screenshots-url=https://dl.flathub.org/media/ --repo=repo builddir build-aux/app.fotema.Fotema.json

# Install Fedora development dependencies
setup:
    sudo dnf install -y libavformat-free-devel
    sudo dnf install -y libavfilter-free-devel
    sudo dnf install -y libavdevice-free-devel
    sudo dnf install -y clang-libs
    sudo dnf install -y clang-devel
    sudo dnf install -y libadwaita-devel
