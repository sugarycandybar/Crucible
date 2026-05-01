project('crucible',
  version: '1.0.0',
  meson_version: '>= 0.59.0'
)

gnome = import('gnome')

prefix = get_option('prefix')
bindir = join_paths(prefix, get_option('bindir'))
datadir = join_paths(prefix, get_option('datadir'))
pkgdatadir = join_paths(datadir, 'crucible')

subdir('crucible')
subdir('packaging')

gnome.post_install(
  glib_compile_schemas: true,
  update_desktop_database: true,
)
