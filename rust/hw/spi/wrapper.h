/*
 * Rust SPI bindgen wrapper.
 *
 * SPDX-License-Identifier: GPL-2.0-or-later
 */

#ifndef __CLANG_STDATOMIC_H
#define __CLANG_STDATOMIC_H
typedef enum memory_order {
  memory_order_relaxed,
  memory_order_consume,
  memory_order_acquire,
  memory_order_release,
  memory_order_acq_rel,
  memory_order_seq_cst,
} memory_order;
#endif

#include "qemu/osdep.h"
#include "hw/ssi/ssi.h"
