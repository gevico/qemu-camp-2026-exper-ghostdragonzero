/*
 * Gevico GPIO test for G233 board
 *
 * Copyright (c) 2025
 *
 * This work is licensed under the terms of the GNU GPL, version 2 or later.
 * See the COPYING file in the top-level directory.
 */

#include "qemu/osdep.h"
#include "libqtest.h"

/* Gevico GPIO registers */
#define GEVICO_GPIO_DIR     0x00  /* Direction Register */
#define GEVICO_GPIO_OUT     0x04  /* Output Data Register */
#define GEVICO_GPIO_IN      0x08  /* Input Data Register */
#define GEVICO_GPIO_IE      0x0C  /* Interrupt Enable Register */
#define GEVICO_GPIO_IS      0x10  /* Interrupt Status Register */
#define GEVICO_GPIO_TRIG    0x14  /* Interrupt Trigger Type Register */
#define GEVICO_GPIO_POL     0x18  /* Interrupt Polarity Register */

/* GPIO base address for G233 */
#define GPIO_BASE 0x10012000

static void test_gevico_gpio_basic_register_access(void)
{
    QTestState *qts = qtest_init("-M g233 -nographic");

    g_print("\n=== Testing Gevico GPIO Basic Register Access ===\n");

    /* Test reading default values */
    uint32_t dir = qtest_readl(qts, GPIO_BASE + GEVICO_GPIO_DIR);
    g_print("GPIO_DIR (default): 0x%08x\n", dir);
    g_assert(dir == 0x00000000);

    uint32_t out = qtest_readl(qts, GPIO_BASE + GEVICO_GPIO_OUT);
    g_print("GPIO_OUT (default): 0x%08x\n", out);
    g_assert(out == 0x00000000);

    uint32_t in = qtest_readl(qts, GPIO_BASE + GEVICO_GPIO_IN);
    g_print("GPIO_IN (default): 0x%08x\n", in);
    g_assert(in == 0x00000000);

    uint32_t ie = qtest_readl(qts, GPIO_BASE + GEVICO_GPIO_IE);
    g_print("GPIO_IE (default): 0x%08x\n", ie);
    g_assert(ie == 0x00000000);

    uint32_t trig = qtest_readl(qts, GPIO_BASE + GEVICO_GPIO_TRIG);
    g_print("GPIO_TRIG (default): 0x%08x\n", trig);
    g_assert(trig == 0x00000000);

    uint32_t pol = qtest_readl(qts, GPIO_BASE + GEVICO_GPIO_POL);
    g_print("GPIO_POL (default): 0x%08x\n", pol);
    g_assert(pol == 0x00000000);

    g_print("✓ Basic register access test passed!\n\n");
    qtest_quit(qts);
}

static void test_gevico_gpio_direction_control(void)
{
    QTestState *qts = qtest_init("-M g233 -nographic");

    g_print("=== Testing Gevico GPIO Direction Control ===\n");

    /* Set pins 0-15 as output, pins 16-31 as input */
    qtest_writel(qts, GPIO_BASE + GEVICO_GPIO_DIR, 0x0000FFFF);
    uint32_t dir = qtest_readl(qts, GPIO_BASE + GEVICO_GPIO_DIR);
    g_print("Set DIR to 0x0000FFFF, read back: 0x%08x\n", dir);
    g_assert(dir == 0x0000FFFF);

    /* Set all pins as output */
    qtest_writel(qts, GPIO_BASE + GEVICO_GPIO_DIR, 0xFFFFFFFF);
    dir = qtest_readl(qts, GPIO_BASE + GEVICO_GPIO_DIR);
    g_print("Set DIR to 0xFFFFFFFF, read back: 0x%08x\n", dir);
    g_assert(dir == 0xFFFFFFFF);

    /* Set all pins as input */
    qtest_writel(qts, GPIO_BASE + GEVICO_GPIO_DIR, 0x00000000);
    dir = qtest_readl(qts, GPIO_BASE + GEVICO_GPIO_DIR);
    g_print("Set DIR to 0x00000000, read back: 0x%08x\n", dir);
    g_assert(dir == 0x00000000);

    g_print("✓ Direction control test passed!\n\n");
    qtest_quit(qts);
}

static void test_gevico_gpio_output_data(void)
{
    QTestState *qts = qtest_init("-M g233 -nographic");

    g_print("=== Testing Gevico GPIO Output Data ===\n");

    /* Configure pins as output */
    qtest_writel(qts, GPIO_BASE + GEVICO_GPIO_DIR, 0xFFFFFFFF);

    /* Set output data */
    qtest_writel(qts, GPIO_BASE + GEVICO_GPIO_OUT, 0x12345678);
    uint32_t out = qtest_readl(qts, GPIO_BASE + GEVICO_GPIO_OUT);
    g_print("Write 0x12345678 to GPIO_OUT, read back: 0x%08x\n", out);
    g_assert(out == 0x12345678);

    /* Read input register (should reflect output when DIR=1) */
    uint32_t in = qtest_readl(qts, GPIO_BASE + GEVICO_GPIO_IN);
    g_print("Read GPIO_IN (should equal GPIO_OUT in output mode): 0x%08x\n", in);
    g_assert(in == 0x12345678);

    /* Toggle output */
    qtest_writel(qts, GPIO_BASE + GEVICO_GPIO_OUT, 0x00000001);
    in = qtest_readl(qts, GPIO_BASE + GEVICO_GPIO_IN);
    g_print("Set GPIO_OUT to 0x00000001, read GPIO_IN: 0x%08x\n", in);
    g_assert(in == 0x00000001);

    qtest_writel(qts, GPIO_BASE + GEVICO_GPIO_OUT, 0x00000000);
    in = qtest_readl(qts, GPIO_BASE + GEVICO_GPIO_IN);
    g_print("Set GPIO_OUT to 0x00000000, read GPIO_IN: 0x%08x\n", in);
    g_assert(in == 0x00000000);

    g_print("✓ Output data test passed!\n\n");
    qtest_quit(qts);
}

static void test_gevico_gpio_interrupt_enable(void)
{
    QTestState *qts = qtest_init("-M g233 -nographic");

    g_print("=== Testing Gevico GPIO Interrupt Enable ===\n");

    /* Enable interrupts for different pins */
    qtest_writel(qts, GPIO_BASE + GEVICO_GPIO_IE, 0x00000001);
    uint32_t ie = qtest_readl(qts, GPIO_BASE + GEVICO_GPIO_IE);
    g_print("Set GPIO_IE to 0x00000001, read back: 0x%08x\n", ie);
    g_assert(ie == 0x00000001);

    qtest_writel(qts, GPIO_BASE + GEVICO_GPIO_IE, 0xFFFFFFFF);
    ie = qtest_readl(qts, GPIO_BASE + GEVICO_GPIO_IE);
    g_print("Set GPIO_IE to 0xFFFFFFFF, read back: 0x%08x\n", ie);
    g_assert(ie == 0xFFFFFFFF);

    /* Disable all interrupts */
    qtest_writel(qts, GPIO_BASE + GEVICO_GPIO_IE, 0x00000000);
    ie = qtest_readl(qts, GPIO_BASE + GEVICO_GPIO_IE);
    g_print("Set GPIO_IE to 0x00000000, read back: 0x%08x\n", ie);
    g_assert(ie == 0x00000000);

    g_print("✓ Interrupt enable test passed!\n\n");
    qtest_quit(qts);
}

static void test_gevico_gpio_interrupt_config(void)
{
    QTestState *qts = qtest_init("-M g233 -nographic");

    g_print("=== Testing Gevico GPIO Interrupt Configuration ===\n");

    /* Configure trigger types */
    qtest_writel(qts, GPIO_BASE + GEVICO_GPIO_TRIG, 0x55555555);
    uint32_t trig = qtest_readl(qts, GPIO_BASE + GEVICO_GPIO_TRIG);
    g_print("Set GPIO_TRIG to 0x55555555, read back: 0x%08x\n", trig);
    g_assert(trig == 0x55555555);

    /* Configure polarity */
    qtest_writel(qts, GPIO_BASE + GEVICO_GPIO_POL, 0xAAAAAAAA);
    uint32_t pol = qtest_readl(qts, GPIO_BASE + GEVICO_GPIO_POL);
    g_print("Set GPIO_POL to 0xAAAAAAAA, read back: 0x%08x\n", pol);
    g_assert(pol == 0xAAAAAAAA);

    /* Set all to edge-triggered, rising edge */
    qtest_writel(qts, GPIO_BASE + GEVICO_GPIO_TRIG, 0x00000000);
    qtest_writel(qts, GPIO_BASE + GEVICO_GPIO_POL, 0xFFFFFFFF);
    trig = qtest_readl(qts, GPIO_BASE + GEVICO_GPIO_TRIG);
    pol = qtest_readl(qts, GPIO_BASE + GEVICO_GPIO_POL);
    g_print("Edge-triggered, rising edge: TRIG=0x%08x, POL=0x%08x\n", trig, pol);
    g_assert(trig == 0x00000000 && pol == 0xFFFFFFFF);

    /* Set all to level-triggered, high level */
    qtest_writel(qts, GPIO_BASE + GEVICO_GPIO_TRIG, 0xFFFFFFFF);
    qtest_writel(qts, GPIO_BASE + GEVICO_GPIO_POL, 0xFFFFFFFF);
    trig = qtest_readl(qts, GPIO_BASE + GEVICO_GPIO_TRIG);
    pol = qtest_readl(qts, GPIO_BASE + GEVICO_GPIO_POL);
    g_print("Level-triggered, high level: TRIG=0x%08x, POL=0x%08x\n", trig, pol);
    g_assert(trig == 0xFFFFFFFF && pol == 0xFFFFFFFF);

    g_print("✓ Interrupt configuration test passed!\n\n");
    qtest_quit(qts);
}

static void test_gevico_gpio_interrupt_status(void)
{
    QTestState *qts = qtest_init("-M g233 -nographic");

    g_print("=== Testing Gevico GPIO Interrupt Status ===\n");

    /* Set some interrupt status bits manually */
    qtest_writel(qts, GPIO_BASE + GEVICO_GPIO_IE, 0xFFFFFFFF);
    qtest_writel(qts, GPIO_BASE + GEVICO_GPIO_IS, 0x00000101);
    uint32_t is = qtest_readl(qts, GPIO_BASE + GEVICO_GPIO_IS);
    g_print("Set GPIO_IS to 0x00000101, read back: 0x%08x\n", is);
    g_assert(is == 0x00000101);

    /* Clear interrupt status by writing 1 */
    qtest_writel(qts, GPIO_BASE + GEVICO_GPIO_IS, 0x00000101);
    is = qtest_readl(qts, GPIO_BASE + GEVICO_GPIO_IS);
    g_print("Clear GPIO_IS by writing 0x00000101, read back: 0x%08x\n", is);
    g_assert(is == 0x00000000);

    /* Set and clear individual bits */
    qtest_writel(qts, GPIO_BASE + GEVICO_GPIO_IS, 0x0000FFFF);
    is = qtest_readl(qts, GPIO_BASE + GEVICO_GPIO_IS);
    g_print("Set GPIO_IS to 0x0000FFFF, read back: 0x%08x\n", is);
    g_assert(is == 0x0000FFFF);

    qtest_writel(qts, GPIO_BASE + GEVICO_GPIO_IS, 0x0000FFFF);
    is = qtest_readl(qts, GPIO_BASE + GEVICO_GPIO_IS);
    g_print("Clear GPIO_IS by writing 0x0000FFFF, read back: 0x%08x\n", is);
    g_assert(is == 0x00000000);

    g_print("✓ Interrupt status test passed!\n\n");
    qtest_quit(qts);
}

int main(int argc, char **argv)
{
    int ret;

    g_test_init(&argc, &argv, NULL);

    g_test_add_func("/g233/gevico-gpio/basic_register_access", test_gevico_gpio_basic_register_access);
    g_test_add_func("/g233/gevico-gpio/direction_control", test_gevico_gpio_direction_control);
    g_test_add_func("/g233/gevico-gpio/output_data", test_gevico_gpio_output_data);
    g_test_add_func("/g233/gevico-gpio/interrupt_enable", test_gevico_gpio_interrupt_enable);
    g_test_add_func("/g233/gevico-gpio/interrupt_config", test_gevico_gpio_interrupt_config);
    g_test_add_func("/g233/gevico-gpio/interrupt_status", test_gevico_gpio_interrupt_status);

    ret = g_test_run();

    return ret;
}