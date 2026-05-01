install_data('linux/io.github.sugarycandybar.Crucible.desktop',
  install_dir: join_paths(datadir, 'applications')
)

install_data('linux/io.github.sugarycandybar.Crucible.metainfo.xml',
  install_dir: join_paths(datadir, 'metainfo')
)

install_data('linux/io.github.sugarycandybar.Crucible.svg',
  install_dir: join_paths(datadir, 'icons/hicolor/scalable/apps')
)
install_data('linux/io.github.sugarycandybar.Crucible-symbolic.svg',
  install_dir: join_paths(datadir, 'icons/hicolor/symbolic/apps')
)

gnome.compile_resources(
  'crucible-resources',
  'io.github.sugarycandybar.Crucible.src.gresource.xml',
  gresource_bundle: true,
  install: true,
  install_dir: pkgdatadir
)

install_data('linux/io.github.sugarycandybar.Crucible.Stability.gschema.xml',
  install_dir: join_paths(datadir, 'glib-2.0/schemas')
)
