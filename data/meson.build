install_data('io.github.sugarycandybar.Crucible.desktop',
  install_dir: get_option('datadir') / 'applications'
)

install_data('io.github.sugarycandybar.Crucible.metainfo.xml',
  install_dir: get_option('datadir') / 'metainfo'
)

install_data('io.github.sugarycandybar.Crucible.Stability.gschema.xml',
  install_dir: get_option('datadir') / 'glib-2.0' / 'schemas'
)

compile_schemas = find_program('glib-compile-schemas', required: false, disabler: true)
test('Validate schema file',
     compile_schemas,
     args: ['--strict', '--dry-run', meson.current_source_dir()])

subdir('icons')

gnome = import('gnome')

pkgdatadir = get_option('prefix') / get_option('datadir') / meson.project_name()

gnome.compile_resources('crucible',
  'crucible.gresource.xml',
  gresource_bundle: true,
  install: true,
  install_dir: pkgdatadir,
  source_dir: meson.current_source_dir(),
)

desktop_file_validate = find_program('desktop-file-validate', required: false)
if desktop_file_validate.found()
  test('Validate desktop file', desktop_file_validate, args: [
    meson.current_source_dir() / 'io.github.sugarycandybar.Crucible.desktop'
  ])
endif

appstreamcli = find_program('appstreamcli', required: false)
if appstreamcli.found()
  test('Validate appstream file', appstreamcli, args: [
    'validate', '--no-net', meson.current_source_dir() / 'io.github.sugarycandybar.Crucible.metainfo.xml'
  ])
endif
