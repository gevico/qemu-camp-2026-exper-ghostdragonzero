/*
 * Gevico GPIO Controller Register Definitions
 *
 * Copyright (c) 2025
 *
 * This work is licensed under the terms of the GNU GPL, version 2 or later.
 * See the COPYING file in the top-level directory.
 */

#ifndef GEVICO_GPIO_H
#define GEVICO_GPIO_H

#include "hw/core/sysbus.h"
#include "qom/object.h"

#define TYPE_GEVICO_GPIO "gevico.gpio"
typedef struct GevicoGPIOState GevicoGPIOState;
DECLARE_INSTANCE_CHECKER(GevicoGPIOState, GEVICO_GPIO, TYPE_GEVICO_GPIO)

#define GEVICO_GPIO_PINS 32
#define GEVICO_GPIO_SIZE 0x1000

/* Register offsets */
#define GEVICO_GPIO_DIR     0x00  /* Direction Register */
#define GEVICO_GPIO_OUT     0x04  /* Output Data Register */
#define GEVICO_GPIO_IN      0x08  /* Input Data Register */
#define GEVICO_GPIO_IE      0x0C  /* Interrupt Enable Register */
#define GEVICO_GPIO_IS      0x10  /* Interrupt Status Register */
#define GEVICO_GPIO_TRIG    0x14  /* Interrupt Trigger Type Register */
#define GEVICO_GPIO_POL     0x18  /* Interrupt Polarity Register */

struct GevicoGPIOState {
    /*< private >*/
    SysBusDevice parent_obj;

    /*< public >*/
    MemoryRegion mmio;

    /* GPIO lines */
    qemu_irq irq;                 /* Combined interrupt to PLIC */
    qemu_irq output[GEVICO_GPIO_PINS];  /* GPIO output lines */

    /* Registers */
    uint32_t dir;      /* Direction: 0=input, 1=output */
    uint32_t out;      /* Output data */
    uint32_t in;       /* Input data (actual pin state) */
    uint32_t ie;       /* Interrupt enable */
    uint32_t is;       /* Interrupt status */
    uint32_t trig;     /* Trigger type: 0=edge, 1=level */
    uint32_t pol;      /* Polarity: 0=low/falling, 1=high/rising */

    /* Internal state */
    uint32_t in_mask;  /* External input mask */
    uint32_t pin_state; /* Actual pin state */

    /* Configuration */
    uint32_t ngpio;
};

#endif /* GEVICO_GPIO_H */