/*
 * Gevico SPI Controller Implementation
 *
 * Copyright (c) 2025
 *
 * This work is licensed under the terms of the GNU GPL, version 2 or later.
 * See the COPYING file in the top-level directory.
 */

#include "qemu/osdep.h"
#include "qemu/log.h"
#include "hw/core/irq.h"
#include "hw/ssi/gevico_spi.h"
#include "hw/core/qdev-properties.h"
#include "migration/vmstate.h"
#include "qemu/module.h"

#define SPI_DEBUG(fmt, ...) qemu_log_mask(LOG_GUEST_ERROR, "GEVICO_SPI: " fmt, ## __VA_ARGS__)

#define FIFO_CAPACITY 8

static void gevico_spi_update_irq(GevicoSpiState *s)
{
    int level = 0;

    /* TX empty interrupt */
    if ((s->cr1 & GEVICO_SPI_CR1_TXEIE) && (s->sr & GEVICO_SPI_SR_TXE)) {
        level = 1;
    }

    /* RX not empty interrupt */
    if ((s->cr1 & GEVICO_SPI_CR1_RXNEIE) && (s->sr & GEVICO_SPI_SR_RXNE)) {
        level = 1;
    }

    /* Error interrupt */
    if ((s->cr1 & GEVICO_SPI_CR1_ERRIE) && (s->sr & GEVICO_SPI_SR_OVERRUN)) {
        level = 1;
    }

    qemu_set_irq(s->irq, level);
}

static void gevico_spi_update_status(GevicoSpiState *s)
{
    /* Update TXE (TX empty) */
    if (fifo8_is_empty(&s->tx_fifo)) {
        s->sr |= GEVICO_SPI_SR_TXE;
    } else {
        s->sr &= ~GEVICO_SPI_SR_TXE;
    }

    /* Update RXNE (RX not empty) */
    if (!fifo8_is_empty(&s->rx_fifo)) {
        s->sr |= GEVICO_SPI_SR_RXNE;
    } else {
        s->sr &= ~GEVICO_SPI_SR_RXNE;
    }

    gevico_spi_update_irq(s);
}

static void gevico_spi_flush_txfifo(GevicoSpiState *s)
{
    uint8_t tx;

    while (!fifo8_is_empty(&s->tx_fifo)) {
        tx = fifo8_pop(&s->tx_fifo);

        /* Transmit data through SPI bus */
        uint8_t rx = ssi_transfer(s->spi, tx);

        /* Check for overrun condition */
        bool had_unread_data = !fifo8_is_empty(&s->rx_fifo);

        /* Receive data into RX FIFO */
        if (fifo8_is_full(&s->rx_fifo)) {
            /* Overrun: RX FIFO full, data lost */
            s->sr |= GEVICO_SPI_SR_OVERRUN;
            SPI_DEBUG("RX FIFO overrun, data lost\n");
        } else {
            fifo8_push(&s->rx_fifo, rx);
            /* Set OVERRUN if we had unread data before receiving this byte */
            if (had_unread_data) {
                s->sr |= GEVICO_SPI_SR_OVERRUN;
                SPI_DEBUG("RX overrun: unread data existed\n");
            }
        }

        SPI_DEBUG("Transferred: TX=0x%02x, RX=0x%02x\n", tx, rx);
    }

    gevico_spi_update_status(s);
}

static uint64_t gevico_spi_read(void *opaque, hwaddr offset, unsigned int size)
{
    GevicoSpiState *s = GEVICO_SPI(opaque);
    uint32_t value = 0;

    switch (offset) {
    case GEVICO_SPI_CR1:
        value = s->cr1;
        SPI_DEBUG("Read SPI_CR1 = 0x%08x\n", value);
        break;

    case GEVICO_SPI_CR2:
        value = s->cr2;
        SPI_DEBUG("Read SPI_CR2 = 0x%08x\n", value);
        break;

    case GEVICO_SPI_SR:
        value = s->sr;
        SPI_DEBUG("Read SPI_SR = 0x%08x\n", value);
        break;

    case GEVICO_SPI_DR:
        /* Read from RX FIFO */
        if (!fifo8_is_empty(&s->rx_fifo)) {
            value = fifo8_pop(&s->rx_fifo);
            SPI_DEBUG("Read SPI_DR = 0x%02x\n", value);
        } else {
            SPI_DEBUG("Read SPI_DR (RX empty)\n");
            value = 0xFF; /* Return 0xFF when RX FIFO empty */
        }
        gevico_spi_update_status(s);
        break;

    default:
        qemu_log_mask(LOG_GUEST_ERROR, "%s: Bad offset 0x%" HWADDR_PRIx "\n",
                     __func__, offset);
        break;
    }

    return value;
}

static void gevico_spi_write(void *opaque, hwaddr offset,
                              uint64_t value, unsigned int size)
{
    GevicoSpiState *s = GEVICO_SPI(opaque);

    switch (offset) {
    case GEVICO_SPI_CR1:
        SPI_DEBUG("Write SPI_CR1 = 0x%08lx\n", value);
        s->cr1 = value & 0xFF; /* Only 8 bits used */

        /* Handle SPI enable/disable */
        if (s->cr1 & GEVICO_SPI_CR1_SPE) {
            /* SPI enabled */
            if (s->cr1 & GEVICO_SPI_CR1_MSTR) {
                /* Master mode: set CS lines */
                int cs = s->cr2 & GEVICO_SPI_CR2_CS_MASK;
                for (int i = 0; i < GEVICO_SPI_CS_COUNT; i++) {
                    qemu_set_irq(s->cs_lines[i], (i != cs));
                }
                SPI_DEBUG("Master mode, CS=%d\n", cs);
            }
        } else {
            /* SPI disabled, all CS high */
            for (int i = 0; i < GEVICO_SPI_CS_COUNT; i++) {
                qemu_set_irq(s->cs_lines[i], 1);
            }
            SPI_DEBUG("SPI disabled\n");
        }

        gevico_spi_update_irq(s);
        break;

    case GEVICO_SPI_CR2:
        SPI_DEBUG("Write SPI_CR2 = 0x%08lx\n", value);
        s->cr2 = value & 0x03; /* Only CS select bits */

        /* Update CS lines if SPI is enabled in master mode */
        if ((s->cr1 & GEVICO_SPI_CR1_SPE) && (s->cr1 & GEVICO_SPI_CR1_MSTR)) {
            int cs = s->cr2 & GEVICO_SPI_CR2_CS_MASK;
            for (int i = 0; i < GEVICO_SPI_CS_COUNT; i++) {
                qemu_set_irq(s->cs_lines[i], (i != cs));
            }
            SPI_DEBUG("CS changed to %d\n", cs);
        }
        break;

    case GEVICO_SPI_SR:
        SPI_DEBUG("Write SPI_SR = 0x%08lx\n", value);
        /* Write 1 to clear OVERRUN flag */
        if (value & GEVICO_SPI_SR_OVERRUN) {
            s->sr &= ~GEVICO_SPI_SR_OVERRUN;
        }
        /* RXNE and TXE are read-only, cannot be cleared by write */
        gevico_spi_update_irq(s);
        break;

    case GEVICO_SPI_DR:
        SPI_DEBUG("Write SPI_DR = 0x%02lx\n", value);
        /* Write to TX FIFO */
        if (!(s->cr1 & GEVICO_SPI_CR1_SPE)) {
            /* SPI not enabled, ignore write */
            SPI_DEBUG("SPI not enabled, ignoring write\n");
            break;
        }

        if (fifo8_is_full(&s->tx_fifo)) {
            SPI_DEBUG("TX FIFO full, cannot write\n");
            break;
        }

        fifo8_push(&s->tx_fifo, value & 0xFF);

        /* Flush TX FIFO if in master mode */
        if (s->cr1 & GEVICO_SPI_CR1_MSTR) {
            gevico_spi_flush_txfifo(s);
        }

        gevico_spi_update_status(s);
        break;

    default:
        qemu_log_mask(LOG_GUEST_ERROR, "%s: Bad offset 0x%" HWADDR_PRIx "\n",
                     __func__, offset);
        break;
    }
}

static const MemoryRegionOps gevico_spi_ops = {
    .read = gevico_spi_read,
    .write = gevico_spi_write,
    .endianness = DEVICE_LITTLE_ENDIAN,
    .impl.min_access_size = 4,
    .impl.max_access_size = 4,
};

static void gevico_spi_reset(DeviceState *dev)
{
    GevicoSpiState *s = GEVICO_SPI(dev);

    s->cr1 = 0x00;
    s->cr2 = 0x00;
    s->sr = GEVICO_SPI_SR_TXE; /* TX empty by default */
    s->dr = 0x00;

    fifo8_reset(&s->tx_fifo);
    fifo8_reset(&s->rx_fifo);

    /* All CS lines high (inactive) */
    for (int i = 0; i < GEVICO_SPI_CS_COUNT; i++) {
        qemu_set_irq(s->cs_lines[i], 1);
    }

    SPI_DEBUG("Device reset\n");
}

static const VMStateDescription vmstate_gevico_spi = {
    .name = TYPE_GEVICO_SPI,
    .version_id = 1,
    .minimum_version_id = 1,
    .fields = (const VMStateField[]) {
        VMSTATE_UINT32(cr1, GevicoSpiState),
        VMSTATE_UINT32(cr2, GevicoSpiState),
        VMSTATE_UINT32(sr, GevicoSpiState),
        VMSTATE_UINT32(dr, GevicoSpiState),
        VMSTATE_FIFO8(tx_fifo, GevicoSpiState),
        VMSTATE_FIFO8(rx_fifo, GevicoSpiState),
        VMSTATE_END_OF_LIST()
    }
};

static void gevico_spi_realize(DeviceState *dev, Error **errp)
{
    GevicoSpiState *s = GEVICO_SPI(dev);

    memory_region_init_io(&s->mmio, OBJECT(dev), &gevico_spi_ops, s,
                          TYPE_GEVICO_SPI, 0x1000);

    sysbus_init_mmio(SYS_BUS_DEVICE(dev), &s->mmio);

    /* Initialize IRQ */
    sysbus_init_irq(SYS_BUS_DEVICE(dev), &s->irq);

    /* Initialize CS lines */
    qdev_init_gpio_out(DEVICE(s), s->cs_lines, GEVICO_SPI_CS_COUNT);

    /* Initialize SPI bus */
    s->spi = ssi_create_bus(dev, "spi");

    /* Initialize FIFOs */
    fifo8_create(&s->tx_fifo, FIFO_CAPACITY);
    fifo8_create(&s->rx_fifo, FIFO_CAPACITY);

    SPI_DEBUG("Device realized\n");
}

static void gevico_spi_class_init(ObjectClass *klass, const void *data)
{
    DeviceClass *dc = DEVICE_CLASS(klass);

    dc->vmsd = &vmstate_gevico_spi;
    dc->realize = gevico_spi_realize;
    device_class_set_legacy_reset(dc, gevico_spi_reset);
    dc->desc = "Gevico SPI Controller";
}

static const TypeInfo gevico_spi_info = {
    .name = TYPE_GEVICO_SPI,
    .parent = TYPE_SYS_BUS_DEVICE,
    .instance_size = sizeof(GevicoSpiState),
    .class_init = gevico_spi_class_init
};

static void gevico_spi_register_types(void)
{
    type_register_static(&gevico_spi_info);
}

type_init(gevico_spi_register_types)
