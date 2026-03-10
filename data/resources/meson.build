blueprint_compiler = find_program('blueprint-compiler')

blueprints_dir = meson.current_source_dir() / 'ui'
blueprints_output_dir = meson.current_build_dir() / 'ui'

blueprints_ui = custom_target('blueprints',
  input: files(
    'ui/window.blp',
    'ui/shortcuts.blp',
    'ui/app_detail.blp',
    'ui/app_tile.blp',
    'ui/explore_page.blp',
    'ui/installed_app_row.blp',
    'ui/installed_page.blp',
    'ui/pending_row.blp',
    'ui/screenshot_slot.blp',
    'ui/search_page.blp',
    'ui/updates_page.blp',
  ),
  output: [
    'window.ui',
    'shortcuts.ui',
    'app_detail.ui',
    'app_tile.ui',
    'explore_page.ui',
    'installed_app_row.ui',
    'installed_page.ui',
    'pending_row.ui',
    'screenshot_slot.ui',
    'search_page.ui',
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
  source_dir: [meson.current_build_dir(), meson.project_source_root() / 'data' / 'icons'],
  dependencies: blueprints_ui,
  install: true,
  install_dir: pkgdatadir,
)
