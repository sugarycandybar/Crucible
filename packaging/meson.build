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

scalable_icons = [
  'processor-symbolic.svg',
  'pci-card-symbolic.svg',
  'memory-symbolic.svg',
  'arrow-pointing-at-line-up-symbolic.svg',
  'stop-sign-large-symbolic.svg',
  'thermometer-symbolic.svg'
]

foreach icon : scalable_icons
  install_data(join_paths('linux/scalable/apps', icon),
    install_dir: join_paths(datadir, 'icons/hicolor/scalable/apps')
  )
endforeach

install_data('linux/io.github.sugarycandybar.Crucible.Stability.gschema.xml',
  install_dir: join_paths(datadir, 'glib-2.0/schemas')
)
