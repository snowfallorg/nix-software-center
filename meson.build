project(
  'nix-software-center',
  'rust',
  version: '0.1.0',
  meson_version: '>= 0.59',
  license: 'GPL-3.0',
)

i18n = import('i18n')
gnome = import('gnome')

base_id = 'org.snowflakeos.NixSoftwareCenter'

dependency('glib-2.0', version: '>= 2.86')
dependency('gio-2.0', version: '>= 2.86')
dependency('gtk4', version: '>= 4.20.0')
dependency('libadwaita-1', version: '>= 1.8.0')

find_program('glib-compile-resources')
glib_compile_schemas = find_program('glib-compile-schemas')
desktop_file_validate = find_program('desktop-file-validate', required: false)
appstreamcli = find_program('appstreamcli', required: false)
cargo = find_program('cargo')

version = meson.project_version()

prefix = get_option('prefix')
bindir = prefix / get_option('bindir')
localedir = prefix / get_option('localedir')

datadir = prefix / get_option('datadir')
pkgdatadir = datadir / meson.project_name()
iconsdir = datadir / 'icons'
podir = meson.project_source_root() / 'po'
gettext_package = meson.project_name()

if get_option('profile') == 'development'
  profile = 'Devel'
  vcs_tag = run_command('git', 'rev-parse', '--short', 'HEAD', check: false).stdout().strip()
  if vcs_tag == ''
    version_suffix = '-devel'
  else
    version_suffix = '-@0@'.format(vcs_tag)
  endif
  application_id = '@0@.@1@'.format(base_id, profile)
else
  profile = ''
  version_suffix = ''
  application_id = base_id
endif

subdir('data')
subdir('po')
subdir('src')

gnome.post_install(
  gtk_update_icon_cache: true,
  glib_compile_schemas: true,
  update_desktop_database: true,
)
