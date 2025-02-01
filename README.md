<!--
SPDX-FileCopyrightText: © 2024 David Bliss

SPDX-License-Identifier: GFDL-1.3-or-later
-->
[![Available on Flathub](https://img.shields.io/flathub/downloads/app.fotema.Fotema?logo=flathub&labelColor=77767b&color=4a90d9)](https://flathub.org/apps/app.fotema.Fotema)
[![Translation status](https://hosted.weblate.org/widget/fotema/app/svg-badge.svg)](https://hosted.weblate.org/engage/fotema/)
[![Please do not theme this app](https://stopthemingmy.app/badge.svg)](https://stopthemingmy.app)

# Fotema

__fotema__ (___adj. Esperanto___) "fond of taking photos"

A photo gallery for Linux.

![All Photos View](/data/resources/screenshots/all-photos.png?raw=true "All Photos View")

## Installation
Fotema is available on Flathub.

<a href='https://flathub.org/apps/app.fotema.Fotema'><img width='240' alt='Download on Flathub' src='https://flathub.org/api/badge?locale=en'/></a>

## Building the project

Install `flatpak`, `flatpak-builder`, and [just](https://github.com/casey/just).

Install [pre-commit](https://pre-commit.com).

```shell
dnf install pre-commit
```

Or, alternatively, install pre-commit with [uv](https://github.com/astral-sh/uv):

```shell
uv tool install pre-commit
```

To build a local development release, run:

```shell
just devel
```

## Roadmap
Aspirationally, this is what I want to add to Fotema.

* Machine learning.
	* Face detection and person recognition, supporting features such as naming a face and then generating an album of all faces that are similar. Preferably in pure Rust, but needs be as needs must.
	* Object and animal recognition.
	* Text recognition, supporting such features as copying text out of a photo or searching for photos containing particular text.
* Search by recognized items, such as "all photos of Sven with a dog".
* A dashboard page showcasing people, places, and events.
* Swipe and keyboard support for the photo/video viewer.
* Fewer (preferably zero) Flatpak permissions.
