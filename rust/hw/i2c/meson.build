_libi2c_bindings_inc_rs = rust.bindgen(
  args: bindgen_args_common + [
    '--allowlist-file', meson.project_source_root() / 'include/hw/i2c/i2c.h',
    '--blocklist-file',
      meson.project_source_root() /
      'include/(block\|chardev/|exec/|hw/core/|qemu/|qom/|system/).*',
  ],
  kwargs: bindgen_kwargs,
)

_libi2c_rs = static_library(
  'i2c',
  structured_sources(
    [
      'src/lib.rs',
      'src/bindings.rs',
      'src/bus.rs',
      'src/device.rs',
      'src/register.rs',
    ],
    {'.' : _libi2c_bindings_inc_rs},
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

rust_devices_ss.add(when: 'CONFIG_X_I2C_RUST', if_true: [declare_dependency(
  link_whole: [_libi2c_rs],
  variables: {'crate': 'i2c'},
)])

test('rust-i2c-unit',
    executable(
        'rust-i2c-unit',
        structured_sources(
          [
            'src/lib.rs',
            'src/bindings.rs',
            'src/bus.rs',
            'src/device.rs',
            'src/register.rs',
          ],
          {'.' : _libi2c_bindings_inc_rs},
        ),
        rust_args: ['--test'],
        install: false,
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
        ]),
    args: [
        '--test', '--test-threads', '1',
        '--format', 'pretty',
    ],
    protocol: 'rust',
    suite: ['unit', 'rust'])
