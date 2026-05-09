/*
 * Gevico PWM Controller
 *
 * Copyright (c) 2025
 *
 * This work is licensed under the terms of the GNU GPL, version 2 or later.
 * See the COPYING file in the top-level directory.
 */

#ifndef GEVICO_PWM_H
#define GEVICO_PWM_H

#include "hw/core/sysbus.h"
#include "qemu/timer.h"
#include "qom/object.h"

#define TYPE_GEVICO_PWM "gevico.pwm"
#define GEVICO_PWM(obj) OBJECT_CHECK(GevicoPwmState, (obj), TYPE_GEVICO_PWM)

#define GEVICO_PWM_CHANNELS 4

/* Register offsets */
#define GEVICO_PWM_GLB        0x00  /* Global control/status */
#define GEVICO_PWM_CH0_BASE   0x10  /* Channel 0 base */
#define GEVICO_PWM_CH1_BASE   0x20  /* Channel 1 base */
#define GEVICO_PWM_CH2_BASE   0x30  /* Channel 2 base */
#define GEVICO_PWM_CH3_BASE   0x40  /* Channel 3 base */

/* Channel register offsets */
#define GEVICO_PWM_CH_CTRL   0x00  /* Channel control */
#define GEVICO_PWM_CH_PERIOD 0x04  /* Period value */
#define GEVICO_PWM_CH_DUTY   0x08  /* Duty cycle */
#define GEVICO_PWM_CH_CNT    0x0C  /* Counter (read-only) */

/* PWM_GLB register fields */
#define GEVICO_PWM_GLB_CH_EN(n)   (1u << (n))      /* Channel enable mirror */
#define GEVICO_PWM_GLB_CH_DONE(n) (1u << (4 + (n))) /* Channel done flag (w1c) */

/* PWM_CHn_CTRL register fields */
#define GEVICO_PWM_CTRL_EN   (1u << 0)  /* Enable */
#define GEVICO_PWM_CTRL_POL  (1u << 1)  /* Polarity */

typedef struct GevicoPwmState GevicoPwmState;

/* Timer callback data */
typedef struct GevicoPwmTimerData {
    GevicoPwmState *state;
    int channel;
} GevicoPwmTimerData;

typedef struct GevicoPwmChannel {
    uint32_t ctrl;
    uint32_t period;
    uint32_t duty;
    uint32_t cnt;
    QEMUTimer timer;
    GevicoPwmTimerData timer_data;
    bool done;
} GevicoPwmChannel;

struct GevicoPwmState {
    /* <private> */
    SysBusDevice parent_obj;

    /* <public> */
    MemoryRegion mmio;

    uint32_t glb;  /* Global register */
    GevicoPwmChannel ch[GEVICO_PWM_CHANNELS];

    qemu_irq irq;  /* Combined interrupt */
};

#endif /* GEVICO_PWM_H */
