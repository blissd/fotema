<!--
SPDX-FileCopyrightText: © 2024 David Bliss

SPDX-License-Identifier: GFDL-1.3-or-later
-->
# Fotema

__fotema__ (___adj. Esperanto___) "fond of taking photos"

A photo gallery for Linux.

![All Photos View](/data/resources/screenshots/all-photos.png?raw=true "All Photos View")

## Building the project

Make sure you have `flatpak` and `flatpak-builder` installed. Then run the commands below. Please note that these commands are just for demonstration purposes. Normally this would be handled by your IDE, such as GNOME Builder or VS Code with the Flatpak extension.

```
flatpak install --user org.gnome.Sdk//45 org.freedesktop.Sdk.Extension.rust-stable//23.08 org.gnome.Platform//45 org.freedesktop.Sdk.Extension.llvm16//23.08
flatpak-builder --user flatpak_app build-aux/dev.romantics.Fotema.Devel.json
```

## Running the project

Once the project is build, run the command below. Please note that these commands are just for demonstration purposes. Normally this would be handled by your IDE, such as GNOME Builder or VS Code with the Flatpak extension.

```
flatpak-builder --run flatpak_app build-aux/dev.romantics.Fotema.Devel.json photo-romantic
```

