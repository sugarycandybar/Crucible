install_data('linux/io.github.sugarycandybar.Crucible.desktop',
  install_dir: datadir / 'applications'
)

install_data('linux/io.github.sugarycandybar.Crucible.metainfo.xml',
  install_dir: datadir / 'metainfo'
)

install_data('linux/io.github.sugarycandybar.Crucible.svg',
  install_dir: datadir / 'icons' / 'hicolor' / 'scalable' / 'apps'
)
install_data('linux/io.github.sugarycandybar.Crucible-symbolic.svg',
  install_dir: datadir / 'icons' / 'hicolor' / 'symbolic' / 'apps'
)

gnome.compile_resources(
  'crucible-resources',
  'io.github.sugarycandybar.Crucible.src.gresource.xml',
  gresource_bundle: true,
  install: true,
  install_dir: pkgdatadir
)

install_data('linux/io.github.sugarycandybar.Crucible.Stability.gschema.xml',
  install_dir: datadir / 'glib-2.0' / 'schemas'
)

desktop_file_validate = find_program('desktop-file-validate', required: false)
if desktop_file_validate.found()
  test('Validate desktop file', desktop_file_validate, args: [
    join_paths(meson.current_source_dir(), 'linux', 'io.github.sugarycandybar.Crucible.desktop')
  ])
endif

appstreamcli = find_program('appstreamcli', required: false)
if appstreamcli.found()
  test('Validate appstream file', appstreamcli, args: [
    'validate', '--no-net', join_paths(meson.current_source_dir(), 'linux', 'io.github.sugarycandybar.Crucible.metainfo.xml')
  ])
endif
