/*
 * Gevico GPIO Controller Implementation
 *
 * Copyright (c) 2025
 *
 * This work is licensed under the terms of the GNU GPL, version 2 or later.
 * See the COPYING file in the top-level directory.
 */

#include "qemu/osdep.h"
#include "qemu/log.h"
#include "hw/core/irq.h"
#include "hw/core/qdev-properties.h"
#include "hw/gpio/gevico_gpio.h"
#include "migration/vmstate.h"
#include "qemu/module.h"

/* Disable trace logging for now */
#define GPIO_TRACE(msg...) do {} while(0)
#define GPIO_DEBUG(msg...) qemu_log_mask(LOG_GUEST_ERROR, "GEVICO_GPIO: " msg)

static void gevico_gpio_update_irq(GevicoGPIOState *s)
{
    uint32_t pending = s->is & s->ie;
    qemu_set_irq(s->irq, pending != 0);
    GPIO_TRACE("IRQ update: pending=0x%08x, enabled=0x%08x, irq=%d\n",
               pending, s->ie, pending != 0);
}

static uint32_t gevico_gpio_get_pin_value(GevicoGPIOState *s, int pin)
{
    uint32_t pin_mask = 1 << pin;

    if (s->dir & pin_mask) {
        /* Output mode: return output latch value */
        return (s->out & pin_mask) != 0;
    } else {
        /* Input mode: return actual pin state */
        return (s->pin_state & pin_mask) != 0;
    }
}
//根据要读的哪一位 如果这一位设置成了out就读out的值  如果是in就读in寄存器的值


static void gevico_gpio_check_interrupt(GevicoGPIOState *s, int pin)
{
    uint32_t pin_mask = 1 << pin;
    uint32_t pin_value = gevico_gpio_get_pin_value(s, pin);
    bool should_trigger = false;

    if (!(s->ie & pin_mask)) {
        /* Interrupt not enabled for this pin */
        return;
    }

    if (s->trig & pin_mask) {
        /* Level-triggered */
        bool level_pol = s->pol & pin_mask; /* 0=low, 1=high */
        should_trigger = (pin_value == level_pol);
        GPIO_TRACE("Pin %d level check: value=%d, pol=%d, trigger=%d\n",
                   pin, pin_value, level_pol, should_trigger);
    } else {
        /* Edge-triggered - this is handled in set function */
        return;
    }

    if (should_trigger) {
        s->is |= pin_mask;
        GPIO_TRACE("Pin %d interrupt triggered, IS=0x%08x\n", pin, s->is);
    } else {
        s->is &= ~pin_mask;
    }

    gevico_gpio_update_irq(s);
}

static void gevico_gpio_update_state(GevicoGPIOState *s)
{
    uint32_t old_pin_state = s->pin_state;

    for (int pin = 0; pin < s->ngpio; pin++) {
        uint32_t pin_mask = 1 << pin;

        /* Update output lines if configured as output */
        if (s->dir & pin_mask) {
            uint32_t out_value = (s->out & pin_mask) != 0;
            GPIO_TRACE("Setting output pin %d to %d\n", pin, out_value);
            qemu_set_irq(s->output[pin], out_value);
            /* Update pin_state to match output in output mode */
            if (out_value) {
                s->pin_state |= pin_mask;
            } else {
                s->pin_state &= ~pin_mask;
            }
        }
    }

    /* Check for edge-triggered interrupts on all pins */
    uint32_t changed = old_pin_state ^ s->pin_state;
    //这个是所有的输出状态有没有改变
    for (int pin = 0; pin < s->ngpio; pin++) {
        uint32_t pin_mask = 1 << pin;

        if (!(changed & pin_mask)) {
            continue; /* No change on this pin */
        }

        if (!(s->ie & pin_mask)) {
            continue; /* Interrupt not enabled */
        }

        if (s->trig & pin_mask) {
            continue; /* Level-triggered, handled separately */
        }

        /* Edge-triggered */
        bool rising = (s->pin_state & pin_mask) != 0;
        bool pol_rising = (s->pol & pin_mask) != 0;

        if (rising == pol_rising) {
            /* Trigger on matching edge */
            s->is |= pin_mask;
            GPIO_TRACE("Pin %d edge interrupt (rising=%d, pol=%d), IS=0x%08x\n",
                       pin, rising, pol_rising, s->is);
        }

        gevico_gpio_update_irq(s);
    }

    /* Check level-triggered interrupts */
    for (int pin = 0; pin < s->ngpio; pin++) {
        uint32_t pin_mask = 1 << pin;
        if ((s->ie & pin_mask) && (s->trig & pin_mask)) {
            gevico_gpio_check_interrupt(s, pin);
        }
    }
}

static uint64_t gevico_gpio_read(void *opaque, hwaddr offset, unsigned int size)
{
    GevicoGPIOState *s = GEVICO_GPIO(opaque);
    uint64_t value = 0;

    switch (offset) {
    case GEVICO_GPIO_DIR:
        value = s->dir;
        break;
    case GEVICO_GPIO_OUT:
        value = s->out;
        break;
    case GEVICO_GPIO_IN:
        /* Read current pin values */
        value = 0;
        for (int i = 0; i < s->ngpio; i++) {
            value |= (gevico_gpio_get_pin_value(s, i) << i);
        }
        break;
    case GEVICO_GPIO_IE:
        value = s->ie;
        break;
    case GEVICO_GPIO_IS:
        value = s->is;
        break;
    case GEVICO_GPIO_TRIG:
        value = s->trig;
        break;
    case GEVICO_GPIO_POL:
        value = s->pol;
        break;
    default:
        qemu_log_mask(LOG_GUEST_ERROR,
                     "%s: Bad read offset 0x%" HWADDR_PRIx "\n",
                     __func__, offset);
        value = 0;
        break;
    }

    GPIO_TRACE("Read 0x%" HWADDR_PRIx " = 0x%08" PRIx64 "\n", offset, value);
    return value;
}

static void gevico_gpio_write(void *opaque, hwaddr offset,
                               uint64_t value, unsigned int size)
{
    GevicoGPIOState *s = GEVICO_GPIO(opaque);

    GPIO_TRACE("Write 0x%" HWADDR_PRIx " = 0x%08" PRIx64 "\n", offset, value);

    switch (offset) {
    case GEVICO_GPIO_DIR:
        s->dir = value;
        GPIO_TRACE("DIR set to 0x%08" PRIx32 "\n", s->dir);
        gevico_gpio_update_state(s);
        break;

    case GEVICO_GPIO_OUT:
        s->out = value;
        GPIO_TRACE("OUT set to 0x%08" PRIx32 "\n", s->out);
        gevico_gpio_update_state(s);
        break;

    case GEVICO_GPIO_IE:
        s->ie = value;
        GPIO_TRACE("IE set to 0x%08" PRIx32 "\n", s->ie);
        gevico_gpio_update_irq(s);
        //这里为什么要触发中断 这个不是一个中断使能寄存器吗
        break;

    case GEVICO_GPIO_IS:
        /* Write 1 to clear interrupt status */
        s->is &= ~value;
        GPIO_TRACE("IS cleared by 0x%08" PRIx64 ", remaining=0x%08" PRIx32 "\n", value, s->is);
        gevico_gpio_update_irq(s);
        break;

    case GEVICO_GPIO_TRIG:
        s->trig = value;
        GPIO_TRACE("TRIG set to 0x%08" PRIx32 "\n", s->trig);
        gevico_gpio_update_state(s);
        break;

    case GEVICO_GPIO_POL:
        s->pol = value;
        GPIO_TRACE("POL set to 0x%08" PRIx32 "\n", s->pol);
        gevico_gpio_update_state(s);
        break;

    default:
        qemu_log_mask(LOG_GUEST_ERROR,
                     "%s: Bad write offset 0x%" HWADDR_PRIx "\n",
                     __func__, offset);
        break;
    }
}

static const MemoryRegionOps gevico_gpio_ops = {
    .read = gevico_gpio_read,
    .write = gevico_gpio_write,
    .endianness = DEVICE_LITTLE_ENDIAN,
    .impl.min_access_size = 4,
    .impl.max_access_size = 4,
};

static void gevico_gpio_set(void *opaque, int line, int value)
{
    GevicoGPIOState *s = GEVICO_GPIO(opaque);

    GPIO_TRACE("External set pin %d to %d\n", line, value);

    assert(line >= 0 && line < GEVICO_GPIO_PINS);

    /* Update pin state */
    uint32_t old_state = s->pin_state;
    s->in_mask |= (1 << line);
    if (value >= 0) {
        if (value) {
            s->pin_state |= (1 << line);
        } else {
            s->pin_state &= ~(1 << line);
        }
    }

    /* Check for edge-triggered interrupts */
    uint32_t changed = old_state ^ s->pin_state;
    for (int pin = 0; pin < s->ngpio; pin++) {
        uint32_t pin_mask = 1 << pin;

        if (!(changed & pin_mask)) {
            continue; /* No change on this pin */
        }

        if (!(s->ie & pin_mask)) {
            continue; /* Interrupt not enabled */
        }

        if (s->trig & pin_mask) {
            continue; /* Level-triggered, handled separately */
        }

        /* Edge-triggered */
        bool rising = (s->pin_state & pin_mask) != 0;
        bool pol_rising = (s->pol & pin_mask) != 0;

        if (rising == pol_rising) {
            /* Trigger on matching edge */
            s->is |= pin_mask;
            GPIO_TRACE("Pin %d edge interrupt (rising=%d, pol=%d), IS=0x%08x\n",
                       pin, rising, pol_rising, s->is);
        }
    }

    gevico_gpio_update_irq(s);
    gevico_gpio_update_state(s);
}

static void gevico_gpio_reset(DeviceState *dev)
{
    GevicoGPIOState *s = GEVICO_GPIO(dev);

    s->dir = 0x00000000;
    s->out = 0x00000000;
    s->in = 0x00000000;
    s->ie = 0x00000000;
    s->is = 0x00000000;
    s->trig = 0x00000000;
    s->pol = 0x00000000;
    s->in_mask = 0x00000000;
    s->pin_state = 0x00000000;

    GPIO_TRACE("Device reset\n");
}

static const VMStateDescription vmstate_gevico_gpio = {
    .name = TYPE_GEVICO_GPIO,
    .version_id = 1,
    .minimum_version_id = 1,
    .fields = (const VMStateField[]) {
        VMSTATE_UINT32(dir, GevicoGPIOState),
        VMSTATE_UINT32(out, GevicoGPIOState),
        VMSTATE_UINT32(in, GevicoGPIOState),
        VMSTATE_UINT32(ie, GevicoGPIOState),
        VMSTATE_UINT32(is, GevicoGPIOState),
        VMSTATE_UINT32(trig, GevicoGPIOState),
        VMSTATE_UINT32(pol, GevicoGPIOState),
        VMSTATE_UINT32(in_mask, GevicoGPIOState),
        VMSTATE_UINT32(pin_state, GevicoGPIOState),
        VMSTATE_END_OF_LIST()
    }
};

static Property gevico_gpio_properties[] = {
    DEFINE_PROP_UINT32("ngpio", GevicoGPIOState, ngpio, GEVICO_GPIO_PINS),
};

static void gevico_gpio_realize(DeviceState *dev, Error **errp)
{
    GevicoGPIOState *s = GEVICO_GPIO(dev);

    /* Set default ngpio if not provided */
    if (s->ngpio == 0) {
        s->ngpio = GEVICO_GPIO_PINS;
    }

    memory_region_init_io(&s->mmio, OBJECT(dev), &gevico_gpio_ops, s,
                          TYPE_GEVICO_GPIO, GEVICO_GPIO_SIZE);

    sysbus_init_mmio(SYS_BUS_DEVICE(dev), &s->mmio);

    /* Initialize single combined IRQ */
    sysbus_init_irq(SYS_BUS_DEVICE(dev), &s->irq);

    /* Initialize GPIO output lines */
    qdev_init_gpio_out(DEVICE(s), s->output, s->ngpio);

    /* Initialize GPIO input lines */
    qdev_init_gpio_in(DEVICE(s), gevico_gpio_set, s->ngpio);
    //初始化了

    GPIO_TRACE("Device realized with %d GPIO pins\n", s->ngpio);
}

static void gevico_gpio_class_init(ObjectClass *klass, const void *data)
{
    DeviceClass *dc = DEVICE_CLASS(klass);

    device_class_set_props(dc, gevico_gpio_properties);
    dc->vmsd = &vmstate_gevico_gpio;
    dc->realize = gevico_gpio_realize;
    device_class_set_legacy_reset(dc, gevico_gpio_reset);
    dc->desc = "Gevico GPIO Controller";
}

static const TypeInfo gevico_gpio_info = {
    .name = TYPE_GEVICO_GPIO,
    .parent = TYPE_SYS_BUS_DEVICE,
    .instance_size = sizeof(GevicoGPIOState),
    .class_init = gevico_gpio_class_init
};

static void gevico_gpio_register_types(void)
{
    type_register_static(&gevico_gpio_info);
}

type_init(gevico_gpio_register_types)