_libsifive_gpio_bindings_inc_rs = rust.bindgen(
  args: bindgen_args_common + [
    '--allowlist-file', meson.project_source_root() / 'include/hw/gpio/sifive_gpio.h',
    '--blocklist-file',
      meson.project_source_root() /
      'include/(block\|chardev/|exec/|hw/core/|qemu/|qom/|system/).*',
  ],
  kwargs: bindgen_kwargs,
)

_libsifive_gpio_rs = static_library(
  'sifive_gpio',
  structured_sources(
    [
      'src/lib.rs',
      'src/bindings.rs',
      'src/device.rs',
      'src/registers.rs',
    ],
    {'.' : _libsifive_gpio_bindings_inc_rs},
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
    bilge_rs,
    bilge_impl_rs,
    bits_rs,
    common_rs,
    glib_sys_rs,
  ],
)

rust_devices_ss.add(when: 'CONFIG_X_SIFIVE_GPIO_RUST', if_true: [declare_dependency(
  link_whole: [_libsifive_gpio_rs],
  variables: {'crate': 'sifive_gpio'},
)])
