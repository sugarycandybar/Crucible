project('crucible',
  version: '1.1.0',
  meson_version: '>= 0.59.0'
)

gnome = import('gnome')

prefix = get_option('prefix')
datadir = prefix / get_option('datadir')
pkgdatadir = datadir / meson.project_name()

# Install desktop file
install_data('data/io.github.sugarycandybar.Crucible.desktop',
  install_dir: datadir / 'applications'
)

# Install AppStream metadata
install_data('data/io.github.sugarycandybar.Crucible.metainfo.xml',
  install_dir: datadir / 'metainfo'
)

# Install icons
install_data('data/io.github.sugarycandybar.Crucible.svg',
  install_dir: datadir / 'icons' / 'hicolor' / 'scalable' / 'apps'
)
install_data('data/io.github.sugarycandybar.Hosty.svg',
  install_dir: datadir / 'icons' / 'hicolor' / 'scalable' / 'apps'
)
install_data('data/io.github.sugarycandybar.Crucible-symbolic.svg',
  install_dir: datadir / 'icons' / 'hicolor' / 'symbolic' / 'apps'
)

# Compile and install GResource (icons bundled for the app)
gnome.compile_resources(
  'crucible-resources',
  'data/crucible.gresource.xml',
  gresource_bundle: true,
  install: true,
  install_dir: pkgdatadir
)

# Install GSettings schema
install_data('data/io.github.sugarycandybar.Crucible.Stability.gschema.xml',
  install_dir: datadir / 'glib-2.0' / 'schemas'
)

# Validation tests
desktop_file_validate = find_program('desktop-file-validate', required: false)
if desktop_file_validate.found()
  test('Validate desktop file', desktop_file_validate, args: [
    meson.current_source_dir() / 'data' / 'io.github.sugarycandybar.Crucible.desktop'
  ])
endif

appstreamcli = find_program('appstreamcli', required: false)
if appstreamcli.found()
  test('Validate appstream file', appstreamcli, args: [
    'validate', '--no-net', meson.current_source_dir() / 'data' / 'io.github.sugarycandybar.Crucible.metainfo.xml'
  ])
endif

gnome.post_install(
  glib_compile_schemas: true,
  update_desktop_database: true,
)
