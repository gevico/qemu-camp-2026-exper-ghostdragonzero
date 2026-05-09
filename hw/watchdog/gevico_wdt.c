/*
 * Gevico Watchdog Timer Implementation
 *
 * Copyright (c) 2025
 *
 * This work is licensed under the terms of the GNU GPL, version 2 or later.
 * See the COPYING file in the top-level directory.
 */

#include "qemu/osdep.h"
#include "qemu/log.h"
#include "hw/core/irq.h"
#include "hw/watchdog/gevico_wdt.h"
#include "hw/core/qdev-properties.h"
#include "migration/vmstate.h"
#include "qemu/module.h"

#define WDT_DEBUG(fmt, ...) qemu_log_mask(LOG_GUEST_ERROR, "GEVICO_WDT: " fmt, ## __VA_ARGS__)

static void gevico_wdt_update_irq(GevicoWdtState *s)
{
    bool pending = (s->sr & GEVICO_WDT_SR_TIMEOUT) &&
                   (s->ctrl & GEVICO_WDT_CTRL_INTEN);
    qemu_set_irq(s->irq, pending);
}

static void gevico_wdt_timer_callback(void *opaque)
{
    GevicoWdtState *s = GEVICO_WDT(opaque);

    /* Decrement counter */
    if (s->val > 0) {
        s->val--;

        /* Check if timeout occurred */
        if (s->val == 0) {
            WDT_DEBUG("Timeout occurred, setting TIMEOUT flag\n");
            s->sr |= GEVICO_WDT_SR_TIMEOUT;
            gevico_wdt_update_irq(s);

            /* Stop timer when reaching zero */
            timer_del(&s->timer);

            /* Note: System reset would happen here in real hardware,
             * but in QEMU we just set the timeout flag */
        } else {
            /* Continue counting - schedule next tick (1us) */
            int64_t now = qemu_clock_get_ns(QEMU_CLOCK_VIRTUAL);
            timer_mod(&s->timer, now + 1000);
        }
    }
}

static uint64_t gevico_wdt_read(void *opaque, hwaddr offset, unsigned int size)
{
    GevicoWdtState *s = GEVICO_WDT(opaque);
    uint32_t value = 0;

    switch (offset) {
    case GEVICO_WDT_CTRL:
        value = s->ctrl;
        WDT_DEBUG("Read WDT_CTRL = 0x%08x\n", value);
        break;

    case GEVICO_WDT_LOAD:
        value = s->load;
        WDT_DEBUG("Read WDT_LOAD = 0x%08x\n", value);
        break;

    case GEVICO_WDT_VAL:
        value = s->val;
        WDT_DEBUG("Read WDT_VAL = %u\n", value);
        break;

    case GEVICO_WDT_SR:
        value = s->sr;
        WDT_DEBUG("Read WDT_SR = 0x%08x\n", value);
        break;

    case GEVICO_WDT_KEY:
        qemu_log_mask(LOG_GUEST_ERROR, "%s: Read from write-only KEY register\n", __func__);
        value = 0;
        break;

    default:
        qemu_log_mask(LOG_GUEST_ERROR, "%s: Bad offset 0x%" HWADDR_PRIx "\n",
                     __func__, offset);
        break;
    }

    return value;
}

static void gevico_wdt_write(void *opaque, hwaddr offset,
                              uint64_t value, unsigned int size)
{
    GevicoWdtState *s = GEVICO_WDT(opaque);

    switch (offset) {
    case GEVICO_WDT_CTRL:
        WDT_DEBUG("Write WDT_CTRL = 0x%08lx (locked=%d)\n", value, s->locked);
        if (s->locked) {
            /* Control register is locked, ignore writes */
            WDT_DEBUG("Control register is locked, ignoring write\n");
            return;
        }
        s->ctrl = value & 0x3; /* Only EN and INTEN bits */

        /* Handle enable/disable */
        if (s->ctrl & GEVICO_WDT_CTRL_EN) {
            /* Enable watchdog */
            if (s->val == 0) {
                /* If counter was at zero, reload from LOAD */
                s->val = s->load;
            }
            if (s->val > 0) {
                int64_t now = qemu_clock_get_ns(QEMU_CLOCK_VIRTUAL);
                timer_mod(&s->timer, now + 1000); /* Start in 1us */
                WDT_DEBUG("Watchdog enabled, val=%u\n", s->val);
            }
        } else {
            /* Disable watchdog */
            timer_del(&s->timer);
            WDT_DEBUG("Watchdog disabled\n");
        }
        break;

    case GEVICO_WDT_LOAD:
        WDT_DEBUG("Write WDT_LOAD = %lu\n", value);
        s->load = value;
        /* Update counter if watchdog is not running */
        if (!(s->ctrl & GEVICO_WDT_CTRL_EN)) {
            s->val = s->load;
        }
        break;

    case GEVICO_WDT_VAL:
        qemu_log_mask(LOG_GUEST_ERROR, "%s: Write to read-only VAL register\n", __func__);
        break;

    case GEVICO_WDT_SR:
        WDT_DEBUG("Write WDT_SR = 0x%08lx\n", value);
        /* Write 1 to clear TIMEOUT flag */
        if (value & GEVICO_WDT_SR_TIMEOUT) {
            s->sr &= ~GEVICO_WDT_SR_TIMEOUT;
        }
        gevico_wdt_update_irq(s);
        break;

    case GEVICO_WDT_KEY:
        WDT_DEBUG("Write WDT_KEY = 0x%08lx\n", value);
        if (value == GEVICO_WDT_KEY_FEED) {
            /* Feed watchdog: reload counter */
            s->val = s->load;
            s->sr &= ~GEVICO_WDT_SR_TIMEOUT; /* Clear timeout flag on feed */
            WDT_DEBUG("Watchdog fed, val reloaded to %u\n", s->val);

            /* Restart timer if enabled */
            if ((s->ctrl & GEVICO_WDT_CTRL_EN) && (s->val > 0)) {
                int64_t now = qemu_clock_get_ns(QEMU_CLOCK_VIRTUAL);
                timer_mod(&s->timer, now + 1000);
            }
        } else if (value == GEVICO_WDT_KEY_LOCK) {
            /* Lock control register */
            s->locked = true;
            WDT_DEBUG("Control register locked\n");
        } else {
            qemu_log_mask(LOG_GUEST_ERROR, "%s: Invalid key 0x%08lx\n",
                         __func__, value);
        }
        break;

    default:
        qemu_log_mask(LOG_GUEST_ERROR, "%s: Bad offset 0x%" HWADDR_PRIx "\n",
                     __func__, offset);
        break;
    }
}

static const MemoryRegionOps gevico_wdt_ops = {
    .read = gevico_wdt_read,
    .write = gevico_wdt_write,
    .endianness = DEVICE_LITTLE_ENDIAN,
    .impl.min_access_size = 4,
    .impl.max_access_size = 4,
};

static void gevico_wdt_reset(DeviceState *dev)
{
    GevicoWdtState *s = GEVICO_WDT(dev);

    s->ctrl = 0x00000000;
    s->load = 0x0000FFFF;
    s->val = 0x0000FFFF;
    s->sr = 0x00000000;
    s->locked = false;

    timer_del(&s->timer);

    WDT_DEBUG("Device reset\n");
}

static const VMStateDescription vmstate_gevico_wdt = {
    .name = TYPE_GEVICO_WDT,
    .version_id = 1,
    .minimum_version_id = 1,
    .fields = (const VMStateField[]) {
        VMSTATE_UINT32(ctrl, GevicoWdtState),
        VMSTATE_UINT32(load, GevicoWdtState),
        VMSTATE_UINT32(val, GevicoWdtState),
        VMSTATE_UINT32(sr, GevicoWdtState),
        VMSTATE_BOOL(locked, GevicoWdtState),
        VMSTATE_TIMER(timer, GevicoWdtState),
        VMSTATE_END_OF_LIST()
    }
};

static void gevico_wdt_realize(DeviceState *dev, Error **errp)
{
    GevicoWdtState *s = GEVICO_WDT(dev);

    memory_region_init_io(&s->mmio, OBJECT(dev), &gevico_wdt_ops, s,
                          TYPE_GEVICO_WDT, 0x1000);

    sysbus_init_mmio(SYS_BUS_DEVICE(dev), &s->mmio);

    /* Initialize IRQ */
    sysbus_init_irq(SYS_BUS_DEVICE(dev), &s->irq);

    /* Initialize timer */
    timer_init_ns(&s->timer, QEMU_CLOCK_VIRTUAL,
                 gevico_wdt_timer_callback, s);

    WDT_DEBUG("Device realized\n");
}

static void gevico_wdt_class_init(ObjectClass *klass, const void *data)
{
    DeviceClass *dc = DEVICE_CLASS(klass);

    dc->vmsd = &vmstate_gevico_wdt;
    dc->realize = gevico_wdt_realize;
    device_class_set_legacy_reset(dc, gevico_wdt_reset);
    dc->desc = "Gevico Watchdog Timer";
}

static const TypeInfo gevico_wdt_info = {
    .name = TYPE_GEVICO_WDT,
    .parent = TYPE_SYS_BUS_DEVICE,
    .instance_size = sizeof(GevicoWdtState),
    .class_init = gevico_wdt_class_init
};

static void gevico_wdt_register_types(void)
{
    type_register_static(&gevico_wdt_info);
}

type_init(gevico_wdt_register_types)
