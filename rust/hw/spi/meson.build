_librspi_bindings_inc_rs = rust.bindgen(
  args: bindgen_args_common + [
    '--allowlist-file', meson.project_source_root() / 'include/hw/ssi/ssi.h',
    '--blocklist-file',
      meson.project_source_root() /
      'include/(block\|chardev/|exec/|hw/core/|qemu/|qom/|system/).*',
  ],
  kwargs: bindgen_kwargs,
)

_librspi_rs = static_library(
  'rspi',
  structured_sources(
    [
      'src/lib.rs',
      'src/bindings.rs',
      'src/device.rs',
      'src/registers.rs',
    ],
    {'.' : _librspi_bindings_inc_rs},
  ),
  override_options: ['rust_std=2021', 'build.rust_std=2021'],
  rust_abi: 'rust',
  link_with: [
    _util_rs,
    _migration_rs,
    _bql_rs,
    _qom_rs,
    _system_rs,
    _hwcore_rs,
    _trace_rs
  ],
  dependencies: [
    bits_rs,
    common_rs,
    glib_sys_rs,
  ],
)

rust_devices_ss.add(when: 'CONFIG_X_SPI_RUST', if_true: [declare_dependency(
  link_whole: [_librspi_rs],
  variables: {'crate': 'rspi'},
)])
