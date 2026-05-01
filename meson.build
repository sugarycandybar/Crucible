project('crucible',
  version: '1.0.0',
  meson_version: '>= 0.59.0'
)

gnome = import('gnome')

bindir = get_option('bindir')
datadir = get_option('datadir')
pkgdatadir = datadir / meson.project_name()

subdir('crucible')
subdir('packaging')

gnome.post_install(
  glib_compile_schemas: true,
  update_desktop_database: true,
)
