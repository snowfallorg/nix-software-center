blueprint_compiler = find_program('blueprint-compiler')

blueprints_dir = meson.current_source_dir() / 'ui'
blueprints_output_dir = meson.current_build_dir() / 'ui'

blueprints_ui = custom_target('blueprints',
  input: files(
    'ui/window.blp',
    'ui/shortcuts.blp',
    'ui/explore_page.blp',
    'ui/installed_page.blp',
    'ui/updates_page.blp',
  ),
  output: [
    'window.ui',
    'shortcuts.ui',
    'explore_page.ui',
    'installed_page.ui',
    'updates_page.ui',
  ],
  command: [
    blueprint_compiler, 'batch-compile',
    blueprints_output_dir,
    blueprints_dir,
    '@INPUT@',
  ],
)

# Resources
resources = gnome.compile_resources(
  'resources',
  'resources.gresource.xml',
  gresource_bundle: true,
  source_dir: meson.current_build_dir(),
  dependencies: blueprints_ui,
  install: true,
  install_dir: pkgdatadir,
)
