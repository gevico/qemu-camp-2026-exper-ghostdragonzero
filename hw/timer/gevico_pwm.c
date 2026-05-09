/*
 * Gevico PWM Controller Implementation
 *
 * Copyright (c) 2025
 *
 * This work is licensed under the terms of the GNU GPL, version 2 or later.
 * See the COPYING file in the top-level directory.
 */

#include "qemu/osdep.h"
#include "qemu/log.h"
#include "hw/core/irq.h"
#include "hw/timer/gevico_pwm.h"
#include "hw/core/qdev-properties.h"
#include "migration/vmstate.h"
#include "qemu/module.h"

/* Enable detailed logging */
#define PWM_DEBUG(fmt, ...) qemu_log_mask(LOG_GUEST_ERROR, "GEVICO_PWM: " fmt, ## __VA_ARGS__)

static void gevico_pwm_update_irq(GevicoPwmState *s)
{
    uint32_t pending = s->glb & 0xF0; /* Bits [7:4] are DONE flags */
    qemu_set_irq(s->irq, pending != 0);
}

static void gevico_pwm_channel_update(GevicoPwmState *s, int channel)
{
    GevicoPwmChannel *ch = &s->ch[channel];

    /* Update global enable mirror */
    if (ch->ctrl & GEVICO_PWM_CTRL_EN) {
        s->glb |= GEVICO_PWM_GLB_CH_EN(channel);
    } else {
        s->glb &= ~GEVICO_PWM_GLB_CH_EN(channel);
    }

    /* Update DONE flag in global register */
    if (ch->done) {
        s->glb |= GEVICO_PWM_GLB_CH_DONE(channel);
    }

    gevico_pwm_update_irq(s);
}

static void gevico_pwm_timer_callback(void *opaque)
{
    GevicoPwmTimerData *data = opaque;
    GevicoPwmState *s = data->state;
    int channel = data->channel;
    GevicoPwmChannel *ch = &s->ch[channel];

    /* Increment counter */
    ch->cnt++;

    /* Check if period completed */
    if (ch->cnt >= ch->period) {
        ch->cnt = 0;
        ch->done = true;
        PWM_DEBUG("Channel %d: period completed, setting DONE flag\n", channel);
        gevico_pwm_channel_update(s, channel);
    }

    /* Reschedule timer if channel is enabled */
    if (ch->ctrl & GEVICO_PWM_CTRL_EN) {
        int64_t now = qemu_clock_get_ns(QEMU_CLOCK_VIRTUAL);
        timer_mod(&ch->timer, now + 1000); /* Increment every 1us */
    }
}

static uint64_t gevico_pwm_read(void *opaque, hwaddr offset, unsigned int size)
{
    GevicoPwmState *s = GEVICO_PWM(opaque);
    uint32_t value = 0;

    switch (offset) {
    case GEVICO_PWM_GLB:
        value = s->glb;
        PWM_DEBUG("Read PWM_GLB = 0x%08x\n", value);
        break;

    default:
        /* Channel registers */
        if (offset >= GEVICO_PWM_CH0_BASE && offset < GEVICO_PWM_CH3_BASE + 0x10) {
            int channel = (offset - GEVICO_PWM_CH0_BASE) / 0x10;
            int reg_offset = (offset - GEVICO_PWM_CH0_BASE) % 0x10;

            if (channel >= GEVICO_PWM_CHANNELS) {
                qemu_log_mask(LOG_GUEST_ERROR, "%s: Invalid channel %d\n", __func__, channel);
                return 0;
            }

            GevicoPwmChannel *ch = &s->ch[channel];

            switch (reg_offset) {
            case GEVICO_PWM_CH_CTRL:
                value = ch->ctrl;
                PWM_DEBUG("Read CH%d_CTRL = 0x%08x\n", channel, value);
                break;
            case GEVICO_PWM_CH_PERIOD:
                value = ch->period;
                PWM_DEBUG("Read CH%d_PERIOD = %u\n", channel, value);
                break;
            case GEVICO_PWM_CH_DUTY:
                value = ch->duty;
                PWM_DEBUG("Read CH%d_DUTY = %u\n", channel, value);
                break;
            case GEVICO_PWM_CH_CNT:
                value = ch->cnt;
                PWM_DEBUG("Read CH%d_CNT = %u\n", channel, value);
                break;
            default:
                qemu_log_mask(LOG_GUEST_ERROR, "%s: Bad offset 0x%" HWADDR_PRIx "\n",
                             __func__, offset);
                return 0;
            }
        } else {
            qemu_log_mask(LOG_GUEST_ERROR, "%s: Bad offset 0x%" HWADDR_PRIx "\n",
                         __func__, offset);
            return 0;
        }
        break;
    }

    return value;
}

static void gevico_pwm_write(void *opaque, hwaddr offset,
                              uint64_t value, unsigned int size)
{
    GevicoPwmState *s = GEVICO_PWM(opaque);

    switch (offset) {
    case GEVICO_PWM_GLB:
        PWM_DEBUG("Write PWM_GLB = 0x%08lx\n", value);
        /* Write 1 to clear DONE flags */
        s->glb &= ~(value & 0xF0); /* Clear bits where write has 1 */
        /* Keep enable mirror bits read-only */
        gevico_pwm_update_irq(s);
        break;

    default:
        /* Channel registers */
        if (offset >= GEVICO_PWM_CH0_BASE && offset < GEVICO_PWM_CH3_BASE + 0x10) {
            int channel = (offset - GEVICO_PWM_CH0_BASE) / 0x10;
            int reg_offset = (offset - GEVICO_PWM_CH0_BASE) % 0x10;

            if (channel >= GEVICO_PWM_CHANNELS) {
                qemu_log_mask(LOG_GUEST_ERROR, "%s: Invalid channel %d\n", __func__, channel);
                return;
            }

            GevicoPwmChannel *ch = &s->ch[channel];
            bool was_enabled = ch->ctrl & GEVICO_PWM_CTRL_EN;

            switch (reg_offset) {
            case GEVICO_PWM_CH_CTRL:
                PWM_DEBUG("Write CH%d_CTRL = 0x%08lx\n", channel, value);
                ch->ctrl = value & 0x3; /* Only EN and POL bits */
                gevico_pwm_channel_update(s, channel);

                /* Handle enable/disable */
                if ((ch->ctrl & GEVICO_PWM_CTRL_EN) && !was_enabled) {
                    /* Channel enabled: start timer */
                    ch->cnt = 0;
                    ch->done = false;
                    int64_t now = qemu_clock_get_ns(QEMU_CLOCK_VIRTUAL);
                    timer_mod(&ch->timer, now + 1000); /* Start in 1us */
                    PWM_DEBUG("Channel %d: enabled\n", channel);
                } else if (!(ch->ctrl & GEVICO_PWM_CTRL_EN) && was_enabled) {
                    /* Channel disabled: stop timer */
                    timer_del(&ch->timer);
                    PWM_DEBUG("Channel %d: disabled\n", channel);
                }
                break;

            case GEVICO_PWM_CH_PERIOD:
                PWM_DEBUG("Write CH%d_PERIOD = %lu\n", channel, value);
                ch->period = value;
                break;

            case GEVICO_PWM_CH_DUTY:
                PWM_DEBUG("Write CH%d_DUTY = %lu\n", channel, value);
                ch->duty = value;
                break;

            case GEVICO_PWM_CH_CNT:
                /* Read-only, ignore writes */
                qemu_log_mask(LOG_GUEST_ERROR, "%s: Write to read-only CNT register\n", __func__);
                break;

            default:
                qemu_log_mask(LOG_GUEST_ERROR, "%s: Bad offset 0x%" HWADDR_PRIx "\n",
                             __func__, offset);
                break;
            }
        } else {
            qemu_log_mask(LOG_GUEST_ERROR, "%s: Bad offset 0x%" HWADDR_PRIx "\n",
                         __func__, offset);
        }
        break;
    }
}

static const MemoryRegionOps gevico_pwm_ops = {
    .read = gevico_pwm_read,
    .write = gevico_pwm_write,
    .endianness = DEVICE_LITTLE_ENDIAN,
    .impl.min_access_size = 4,
    .impl.max_access_size = 4,
};

static void gevico_pwm_reset(DeviceState *dev)
{
    GevicoPwmState *s = GEVICO_PWM(dev);

    s->glb = 0x00000000;

    for (int i = 0; i < GEVICO_PWM_CHANNELS; i++) {
        s->ch[i].ctrl = 0x00000000;
        s->ch[i].period = 0x00000000;
        s->ch[i].duty = 0x00000000;
        s->ch[i].cnt = 0x00000000;
        s->ch[i].done = false;
        timer_del(&s->ch[i].timer);
    }

    PWM_DEBUG("Device reset\n");
}

static const VMStateDescription vmstate_gevico_pwm_channel = {
    .name = "gevico-pwm-channel",
    .version_id = 1,
    .minimum_version_id = 1,
    .fields = (const VMStateField[]) {
        VMSTATE_UINT32(ctrl, GevicoPwmChannel),
        VMSTATE_UINT32(period, GevicoPwmChannel),
        VMSTATE_UINT32(duty, GevicoPwmChannel),
        VMSTATE_UINT32(cnt, GevicoPwmChannel),
        VMSTATE_BOOL(done, GevicoPwmChannel),
        VMSTATE_TIMER(timer, GevicoPwmChannel),
        VMSTATE_END_OF_LIST()
    }
};

static const VMStateDescription vmstate_gevico_pwm = {
    .name = TYPE_GEVICO_PWM,
    .version_id = 1,
    .minimum_version_id = 1,
    .fields = (const VMStateField[]) {
        VMSTATE_UINT32(glb, GevicoPwmState),
        VMSTATE_STRUCT_ARRAY(ch, GevicoPwmState, GEVICO_PWM_CHANNELS, 0,
                            vmstate_gevico_pwm_channel, GevicoPwmChannel),
        VMSTATE_END_OF_LIST()
    }
};

static void gevico_pwm_realize(DeviceState *dev, Error **errp)
{
    GevicoPwmState *s = GEVICO_PWM(dev);

    memory_region_init_io(&s->mmio, OBJECT(dev), &gevico_pwm_ops, s,
                          TYPE_GEVICO_PWM, 0x1000);

    sysbus_init_mmio(SYS_BUS_DEVICE(dev), &s->mmio);

    /* Initialize combined IRQ */
    sysbus_init_irq(SYS_BUS_DEVICE(dev), &s->irq);

    /* Initialize timers for each channel */
    for (int i = 0; i < GEVICO_PWM_CHANNELS; i++) {
        s->ch[i].timer_data.state = s;
        s->ch[i].timer_data.channel = i;
        timer_init_ns(&s->ch[i].timer, QEMU_CLOCK_VIRTUAL,
                     gevico_pwm_timer_callback, &s->ch[i].timer_data);
    }

    PWM_DEBUG("Device realized with %d channels\n", GEVICO_PWM_CHANNELS);
}

static void gevico_pwm_class_init(ObjectClass *klass, const void *data)
{
    DeviceClass *dc = DEVICE_CLASS(klass);

    dc->vmsd = &vmstate_gevico_pwm;
    dc->realize = gevico_pwm_realize;
    device_class_set_legacy_reset(dc, gevico_pwm_reset);
    dc->desc = "Gevico PWM Controller";
}

static const TypeInfo gevico_pwm_info = {
    .name = TYPE_GEVICO_PWM,
    .parent = TYPE_SYS_BUS_DEVICE,
    .instance_size = sizeof(GevicoPwmState),
    .class_init = gevico_pwm_class_init
};

static void gevico_pwm_register_types(void)
{
    type_register_static(&gevico_pwm_info);
}

type_init(gevico_pwm_register_types)
