/*
 * Simple GPIO test for G233 board
 *
 * Copyright (c) 2025
 *
 * This work is licensed under the terms of the GNU GPL, version 2 or later.
 * See the COPYING file in the top-level directory.
 */

#include "qemu/osdep.h"
#include "libqtest.h"

/* GPIO registers for SiFive GPIO */
#define SIFIVE_GPIO_REG_VALUE      0x000
#define SIFIVE_GPIO_REG_INPUT_EN   0x004
#define SIFIVE_GPIO_REG_OUTPUT_EN  0x008
#define SIFIVE_GPIO_REG_PORT       0x00C
#define SIFIVE_GPIO_REG_PUE        0x010
#define SIFIVE_GPIO_REG_DS         0x014
#define SIFIVE_GPIO_REG_RISE_IE    0x018
#define SIFIVE_GPIO_REG_RISE_IP    0x01C
#define SIFIVE_GPIO_REG_FALL_IE    0x020
#define SIFIVE_GPIO_REG_FALL_IP    0x024
#define SIFIVE_GPIO_REG_HIGH_IE    0x028
#define SIFIVE_GPIO_REG_HIGH_IP    0x02C
#define SIFIVE_GPIO_REG_LOW_IE     0x030
#define SIFIVE_GPIO_REG_LOW_IP     0x034
#define SIFIVE_GPIO_REG_IOF_EN     0x038
#define SIFIVE_GPIO_REG_IOF_SEL    0x03C
#define SIFIVE_GPIO_REG_OUT_XOR    0x040

/* GPIO base address for G233 */
#define GPIO_BASE 0x10012000

static void test_gpio_register_access(void)
{
    QTestState *qts = qtest_init("-M g233 -nographic");

    g_print("\n=== Testing GPIO Register Access ===\n");

    /* Test reading default values */
    g_print("Reading GPIO_OUTPUT_EN (0x10012008): 0x%08x\n",
            qtest_readl(qts, GPIO_BASE + SIFIVE_GPIO_REG_OUTPUT_EN));
    g_print("Reading GPIO_INPUT_EN (0x10012004): 0x%08x\n",
            qtest_readl(qts, GPIO_BASE + SIFIVE_GPIO_REG_INPUT_EN));
    g_print("Reading GPIO_PORT (0x1001200C): 0x%08x\n",
            qtest_readl(qts, GPIO_BASE + SIFIVE_GPIO_REG_PORT));
    g_print("Reading GPIO_VALUE (0x10012000): 0x%08x\n",
            qtest_readl(qts, GPIO_BASE + SIFIVE_GPIO_REG_VALUE));

    /* Test writing to output enable */
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_OUTPUT_EN, 0xFFFFFFFF);
    uint32_t output_en = qtest_readl(qts, GPIO_BASE + SIFIVE_GPIO_REG_OUTPUT_EN);
    g_print("After writing 0xFFFFFFFF to GPIO_OUTPUT_EN: 0x%08x\n", output_en);
    g_assert(output_en == 0xFFFFFFFF);

    /* Test writing to input enable */
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_INPUT_EN, 0x0000FFFF);
    uint32_t input_en = qtest_readl(qts, GPIO_BASE + SIFIVE_GPIO_REG_INPUT_EN);
    g_print("After writing 0x0000FFFF to GPIO_INPUT_EN: 0x%08x\n", input_en);
    g_assert(input_en == 0x0000FFFF);

    /* Test writing to port */
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_PORT, 0x12345678);
    uint32_t port = qtest_readl(qts, GPIO_BASE + SIFIVE_GPIO_REG_PORT);
    g_print("After writing 0x12345678 to GPIO_PORT: 0x%08x\n", port);
    g_assert(port == 0x12345678);

    g_print("✓ GPIO register access test passed!\n\n");

    qtest_quit(qts);
}

static void test_gpio_interrupt_enable(void)
{
    QTestState *qts = qtest_init("-M g233 -nographic");

    g_print("=== Testing GPIO Interrupt Enable ===\n");

    /* Test rise interrupt enable */
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_RISE_IE, 0x00000001);
    uint32_t rise_ie = qtest_readl(qts, GPIO_BASE + SIFIVE_GPIO_REG_RISE_IE);
    g_print("After writing 0x1 to GPIO_RISE_IE: 0x%08x\n", rise_ie);
    g_assert(rise_ie == 0x00000001);

    /* Test fall interrupt enable */
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_FALL_IE, 0x00000002);
    uint32_t fall_ie = qtest_readl(qts, GPIO_BASE + SIFIVE_GPIO_REG_FALL_IE);
    g_print("After writing 0x2 to GPIO_FALL_IE: 0x%08x\n", fall_ie);
    g_assert(fall_ie == 0x00000002);

    /* Test high interrupt enable */
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_HIGH_IE, 0x00000004);
    uint32_t high_ie = qtest_readl(qts, GPIO_BASE + SIFIVE_GPIO_REG_HIGH_IE);
    g_print("After writing 0x4 to GPIO_HIGH_IE: 0x%08x\n", high_ie);
    g_assert(high_ie == 0x00000004);

    /* Test low interrupt enable */
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_LOW_IE, 0x00000008);
    uint32_t low_ie = qtest_readl(qts, GPIO_BASE + SIFIVE_GPIO_REG_LOW_IE);
    g_print("After writing 0x8 to GPIO_LOW_IE: 0x%08x\n", low_ie);
    g_assert(low_ie == 0x00000008);

    g_print("✓ GPIO interrupt enable test passed!\n\n");

    qtest_quit(qts);
}

static void test_gpio_output_toggle(void)
{
    QTestState *qts = qtest_init("-M g233 -nographic");

    g_print("=== Testing GPIO Output Toggle ===\n");

    /* Enable pin 0 as output */
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_OUTPUT_EN, 0x00000001);
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_INPUT_EN, 0x00000000);

    /* Set pin 0 high */
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_PORT, 0x00000001);
    uint32_t value1 = qtest_readl(qts, GPIO_BASE + SIFIVE_GPIO_REG_VALUE);
    g_print("Pin 0 set high, GPIO_VALUE: 0x%08x\n", value1);

    /* Set pin 0 low */
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_PORT, 0x00000000);
    uint32_t value2 = qtest_readl(qts, GPIO_BASE + SIFIVE_GPIO_REG_VALUE);
    g_print("Pin 0 set low, GPIO_VALUE: 0x%08x\n", value2);

    /* Toggle multiple pins */
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_OUTPUT_EN, 0xFFFFFFFF);
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_PORT, 0x55555555);
    uint32_t value3 = qtest_readl(qts, GPIO_BASE + SIFIVE_GPIO_REG_PORT);
    g_print("Wrote 0x55555555 to GPIO_PORT, read back: 0x%08x\n", value3);
    g_assert(value3 == 0x55555555);

    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_PORT, 0xAAAAAAAA);
    uint32_t value4 = qtest_readl(qts, GPIO_BASE + SIFIVE_GPIO_REG_PORT);
    g_print("Wrote 0xAAAAAAAA to GPIO_PORT, read back: 0x%08x\n", value4);
    g_assert(value4 == 0xAAAAAAAA);

    g_print("✓ GPIO output toggle test passed!\n\n");

    qtest_quit(qts);
}

int main(int argc, char **argv)
{
    int ret;

    g_test_init(&argc, &argv, NULL);

    g_test_add_func("/g233/gpio/register_access", test_gpio_register_access);
    g_test_add_func("/g233/gpio/interrupt_enable", test_gpio_interrupt_enable);
    g_test_add_func("/g233/gpio/output_toggle", test_gpio_output_toggle);

    ret = g_test_run();

    return ret;
}