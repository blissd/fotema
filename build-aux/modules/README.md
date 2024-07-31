# Modules

## libonnxruntime
Required by the rust-faces dependencies. The rust-faces cargo build
actually puts a libonnxruntime.so in the build tree... but I can't get
Meson to find the file and copy to the /app/lib directory of the Flatpak.

If that can be fixed, then this module can be removed.
