builddir := "builddir"
prefix := justfile_directory() / builddir / "install"
profile := "development"

# Configure meson build directory
setup:
    @if [ ! -f {{builddir}}/build.ninja ]; then \
        meson setup {{builddir}} -Dprofile={{profile}} -Dprefix={{prefix}}; \
    elif ! meson configure {{builddir}} | grep -q "profile.*{{profile}}"; then \
        meson setup {{builddir}} --reconfigure -Dprofile={{profile}} -Dprefix={{prefix}}; \
    fi

# Reconfigure existing build directory
reconfigure:
    meson setup {{builddir}} --reconfigure -Dprofile={{profile}} -Dprefix={{prefix}}

# Build the project
build: setup
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

# Watch for changes and rebuild
watch:
    bacon

# Run clippy lints
lint:
    cargo clippy --manifest-path Cargo.toml

# Format code
fmt:
    cargo fmt --manifest-path Cargo.toml

# Fix lints and format code
fix:
    cargo clippy --manifest-path Cargo.toml --fix --allow-dirty --allow-staged
    cargo fmt --manifest-path Cargo.toml

# Clean and reconfigure from scratch
rebuild: clean setup build
