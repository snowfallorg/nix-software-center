builddir := "builddir"
prefix := justfile_directory() / builddir / "install"
profile := "development"

# Configure meson build directory
setup:
    meson setup {{builddir}} -Dprofile={{profile}} -Dprefix={{prefix}}

# Reconfigure existing build directory
reconfigure:
    meson setup {{builddir}} --reconfigure -Dprofile={{profile}} -Dprefix={{prefix}}

# Build the project
build:
    meson compile -C {{builddir}}

# Install to local prefix
install: build
    meson install -C {{builddir}}

# Build, install, and run the app
run: install
    RUST_LOG=nix_software_center=DEBUG \
    GSETTINGS_SCHEMA_DIR={{prefix}}/share/glib-2.0/schemas \
    XDG_DATA_DIRS="{{prefix}}/share:${XDG_DATA_DIRS}" \
    {{prefix}}/bin/nix-software-center

# Clean build directory
clean:
    rm -rf {{builddir}}

# Clean and reconfigure from scratch
rebuild: clean setup build
