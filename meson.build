project('crucible',
  version: '1.0.0',
  meson_version: '>= 0.59.0'
)

gnome = import('gnome')

prefix = get_option('prefix')
bindir = prefix / get_option('bindir')
datadir = prefix / get_option('datadir')
localedir = prefix / get_option('localedir')
pkgdatadir = datadir / meson.project_name()

subdir('crucible')
subdir('packaging')

gnome.post_install(
  glib_compile_schemas: true,
  update_desktop_database: true,
)
