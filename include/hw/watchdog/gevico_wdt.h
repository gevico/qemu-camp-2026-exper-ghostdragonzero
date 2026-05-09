/*
 * Gevico Watchdog Timer
 *
 * Copyright (c) 2025
 *
 * This work is licensed under the terms of the GNU GPL, version 2 or later.
 * See the COPYING file in the top-level directory.
 */

#ifndef GEVICO_WDT_H
#define GEVICO_WDT_H

#include "hw/core/sysbus.h"
#include "qemu/timer.h"
#include "qom/object.h"

#define TYPE_GEVICO_WDT "gevico.wdt"
#define GEVICO_WDT(obj) OBJECT_CHECK(GevicoWdtState, (obj), TYPE_GEVICO_WDT)

/* Register offsets */
#define GEVICO_WDT_CTRL   0x00  /* Control register */
#define GEVICO_WDT_LOAD   0x04  /* Load value register */
#define GEVICO_WDT_VAL    0x08  /* Current counter value (read-only) */
#define GEVICO_WDT_KEY    0x0C  /* Key register (write-only) */
#define GEVICO_WDT_SR     0x10  /* Status register */

/* WDT_CTRL register fields */
#define GEVICO_WDT_CTRL_EN     (1u << 0)  /* Enable */
#define GEVICO_WDT_CTRL_INTEN  (1u << 1)  /* Interrupt enable */

/* WDT_KEY register values */
#define GEVICO_WDT_KEY_FEED   0x5A5A5A5A  /* Feed watchdog */
#define GEVICO_WDT_KEY_LOCK   0x1ACCE551  /* Lock control register */

/* WDT_SR register fields */
#define GEVICO_WDT_SR_TIMEOUT  (1u << 0)  /* Timeout flag (write-1-to-clear) */

typedef struct GevicoWdtState {
    /* <private> */
    SysBusDevice parent_obj;

    /* <public> */
    MemoryRegion mmio;

    uint32_t ctrl;     /* Control register */
    uint32_t load;     /* Load value register */
    uint32_t val;      /* Current counter value */
    uint32_t sr;       /* Status register */

    bool locked;       /* Control register locked */

    QEMUTimer timer;   /* Timer for countdown */
    qemu_irq irq;      /* Interrupt line */
} GevicoWdtState;

#endif /* GEVICO_WDT_H */
