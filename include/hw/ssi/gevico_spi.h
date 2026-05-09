/*
 * Gevico SPI Controller
 *
 * Copyright (c) 2025
 *
 * This work is licensed under the terms of the GNU GPL, version 2 or later.
 * See the COPYING file in the top-level directory.
 */

#ifndef GEVICO_SPI_H
#define GEVICO_SPI_H

#include "hw/core/sysbus.h"
#include "hw/ssi/ssi.h"
#include "qemu/fifo8.h"
#include "qom/object.h"

#define TYPE_GEVICO_SPI "gevico.spi"
#define GEVICO_SPI(obj) OBJECT_CHECK(GevicoSpiState, (obj), TYPE_GEVICO_SPI)

#define GEVICO_SPI_CS_COUNT 4

/* Register offsets */
#define GEVICO_SPI_CR1   0x00  /* Control register 1 */
#define GEVICO_SPI_CR2   0x04  /* Control register 2 */
#define GEVICO_SPI_SR    0x08  /* Status register */
#define GEVICO_SPI_DR    0x0C  /* Data register */

/* SPI_CR1 register fields */
#define GEVICO_SPI_CR1_SPE     (1u << 0)  /* SPI enable */
#define GEVICO_SPI_CR1_MSTR    (1u << 2)  /* Master mode */
#define GEVICO_SPI_CR1_ERRIE   (1u << 5)  /* Error interrupt enable */
#define GEVICO_SPI_CR1_RXNEIE  (1u << 6)  /* RX not empty interrupt enable */
#define GEVICO_SPI_CR1_TXEIE   (1u << 7)  /* TX empty interrupt enable */

/* SPI_CR2 register fields */
#define GEVICO_SPI_CR2_CS_MASK  (0x3)     /* CS select mask */

/* SPI_SR register fields */
#define GEVICO_SPI_SR_RXNE    (1u << 0)  /* RX not empty */
#define GEVICO_SPI_SR_TXE     (1u << 1)  /* TX empty */
#define GEVICO_SPI_SR_OVERRUN (1u << 4)  /* Overrun error (w1c) */

typedef struct GevicoSpiState {
    /* <private> */
    SysBusDevice parent_obj;

    /* <public> */
    MemoryRegion mmio;

    uint32_t cr1;     /* Control register 1 */
    uint32_t cr2;     /* Control register 2 */
    uint32_t sr;      /* Status register */
    uint32_t dr;      /* Data register */

    Fifo8 tx_fifo;
    Fifo8 rx_fifo;

    qemu_irq irq;
    qemu_irq cs_lines[GEVICO_SPI_CS_COUNT];
    SSIBus *spi;
} GevicoSpiState;

#endif /* GEVICO_SPI_H */
