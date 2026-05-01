conf = configuration_data()
conf.set('VERSION', meson.project_version())
conf.set('localedir', localedir)
conf.set('pkgdatadir', pkgdatadir)
conf.set('datadir', datadir)

configure_file(
  input: '../crucible.in',
  output: 'crucible',
  configuration: conf,
  install_dir: bindir,
  install_mode: 'rwxr-xr-x'
)

install_subdir('.',
  exclude_files: ['meson.build'],
  exclude_directories: ['__pycache__'],
  install_dir: pkgdatadir,
  strip_directory: true
)
