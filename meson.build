project('crucible',
  version: '1.1.0',
  meson_version: '>= 0.59.0'
)

cargo = find_program('cargo', required: true)

custom_target('crucible',
  output: 'crucible',
  build_always_stale: true,
  build_by_default: true,
  command: [
    'sh', '-c',
    'cargo build --release --offline --manifest-path "$1" --target-dir "$2/target" && cp "$2/target/release/crucible" "$2/crucible"',
    '_',
    meson.current_source_dir() / 'Cargo.toml',
    meson.current_build_dir(),
  ],
  console: true,
  install: true,
  install_dir: get_option('bindir'),
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
  install_dir: pkgdatadir,
  source_dir: 'data',
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
)
