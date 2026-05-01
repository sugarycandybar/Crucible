install_subdir('.',
  exclude_files: ['meson.build'],
  install_dir: pkgdatadir,
  strip_directory: true
)

install_data('../crucible.py',
  install_dir: datadir,
  install_mode: 'rwxr-xr-x'
)

install_data('../packaging/flatpak/crucible-wrapper.sh',
  rename: 'crucible',
  install_dir: bindir,
  install_mode: 'rwxr-xr-x'
)
