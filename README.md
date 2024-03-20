# Photo Romantic

Clean and simple GNOME photo gallery.

## Building the project

Make sure you have `flatpak` and `flatpak-builder` installed. Then run the commands below. Please note that these commands are just for demonstration purposes. Normally this would be handled by your IDE, such as GNOME Builder or VS Code with the Flatpak extension.

```
flatpak install --user org.gnome.Sdk//45 org.freedesktop.Sdk.Extension.rust-stable//23.08 org.gnome.Platform//45 org.freedesktop.Sdk.Extension.llvm16//23.08
flatpak-builder --user flatpak_app build-aux/dev.romantics.Photos.Devel.json
```

## Running the project

Once the project is build, run the command below. Please note that these commands are just for demonstration purposes. Normally this would be handled by your IDE, such as GNOME Builder or VS Code with the Flatpak extension.

```
flatpak-builder --run flatpak_app build-aux/dev.romantics.Photos.Devel.json photo-romantic
```

