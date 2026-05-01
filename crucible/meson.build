install_subdir('.',
  exclude_files: ['meson.build'],
  exclude_directories: ['__pycache__'],
  install_dir: pkgdatadir,
  strip_directory: true
)

install_data('../crucible.py',
  rename: 'main.py',
  install_dir: pkgdatadir,
  install_mode: 'rwxr-xr-x'
)

install_data('../packaging/flatpak/crucible-wrapper.sh',
  rename: 'crucible',
  install_dir: bindir,
  install_mode: 'rwxr-xr-x'
)
