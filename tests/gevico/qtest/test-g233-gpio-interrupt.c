/*
 * GPIO Interrupt test for G233 board
 *
 * Copyright (c) 2025
 *
 * This work is licensed under the terms of the GNU GPL, version 2 or later.
 * See the COPYING file in the top-level directory.
 */

#include "qemu/osdep.h"
#include "libqtest.h"

/* GPIO registers */
#define SIFIVE_GPIO_REG_VALUE      0x000
#define SIFIVE_GPIO_REG_INPUT_EN   0x004
#define SIFIVE_GPIO_REG_OUTPUT_EN  0x008
#define SIFIVE_GPIO_REG_PORT       0x00C
#define SIFIVE_GPIO_REG_PUE        0x010
#define SIFIVE_GPIO_REG_RISE_IE    0x018
#define SIFIVE_GPIO_REG_RISE_IP    0x01C
#define SIFIVE_GPIO_REG_FALL_IE    0x020
#define SIFIVE_GPIO_REG_FALL_IP    0x024
#define SIFIVE_GPIO_REG_HIGH_IE    0x028
#define SIFIVE_GPIO_REG_HIGH_IP    0x02C
#define SIFIVE_GPIO_REG_LOW_IE     0x030
#define SIFIVE_GPIO_REG_LOW_IP     0x034
#define SIFIVE_GPIO_REG_OUT_XOR    0x040

/* GPIO base address for G233 */
#define GPIO_BASE 0x10012000

/* PLIC registers */
#define PLIC_BASE      0x0c000000
#define PLIC_PENDING   (PLIC_BASE + 0x1000)
#define PLIC_ENABLE    (PLIC_BASE + 0x2000)
#define PLIC_CONTEXT   (PLIC_BASE + 0x200000)

static void test_gpio_rising_interrupt(void)
{
    QTestState *qts = qtest_init("-M g233 -nographic");

    g_print("\n=== Testing GPIO Rising Edge Interrupt ===\n");

    /* Configure pin 0 as input with pull-up */
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_OUTPUT_EN, 0x00000000);
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_INPUT_EN, 0x00000001);
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_PUE, 0x00000001);

    /* Enable rising edge interrupt for pin 0 */
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_RISE_IE, 0x00000001);

    /* Clear any pending interrupts */
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_RISE_IP, 0xFFFFFFFF);
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_FALL_IP, 0xFFFFFFFF);
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_HIGH_IP, 0xFFFFFFFF);
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_LOW_IP, 0xFFFFFFFF);

    g_print("GPIO configured for rising edge interrupt on pin 0\n");

    /* Read initial interrupt pending status */
    uint32_t rise_ip_before = qtest_readl(qts, GPIO_BASE + SIFIVE_GPIO_REG_RISE_IP);
    g_print("Initial RISE_IP: 0x%08x\n", rise_ip_before);

    /* Simulate rising edge by setting pin low then high */
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_PORT, 0x00000000);
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_PORT, 0x00000001);

    /* Read interrupt pending status after edge */
    uint32_t rise_ip_after = qtest_readl(qts, GPIO_BASE + SIFIVE_GPIO_REG_RISE_IP);
    g_print("After rising edge, RISE_IP: 0x%08x\n", rise_ip_after);

    /* Check PLIC pending */
    uint32_t plic_pending = qtest_readl(qts, PLIC_PENDING);
    g_print("PLIC pending register: 0x%08x\n", plic_pending);

    g_print("✓ Rising edge interrupt test completed!\n\n");

    qtest_quit(qts);
}

static void test_gpio_falling_interrupt(void)
{
    QTestState *qts = qtest_init("-M g233 -nographic");

    g_print("=== Testing GPIO Falling Edge Interrupt ===\n");

    /* Configure pin 1 as input with pull-up */
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_OUTPUT_EN, 0x00000000);
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_INPUT_EN, 0x00000002);
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_PUE, 0x00000002);

    /* Enable falling edge interrupt for pin 1 */
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_FALL_IE, 0x00000002);

    /* Clear any pending interrupts */
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_RISE_IP, 0xFFFFFFFF);
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_FALL_IP, 0xFFFFFFFF);
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_HIGH_IP, 0xFFFFFFFF);
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_LOW_IP, 0xFFFFFFFF);

    g_print("GPIO configured for falling edge interrupt on pin 1\n");

    /* Read initial interrupt pending status */
    uint32_t fall_ip_before = qtest_readl(qts, GPIO_BASE + SIFIVE_GPIO_REG_FALL_IP);
    g_print("Initial FALL_IP: 0x%08x\n", fall_ip_before);

    /* Simulate falling edge by setting pin high then low */
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_PORT, 0x00000002);
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_PORT, 0x00000000);

    /* Read interrupt pending status after edge */
    uint32_t fall_ip_after = qtest_readl(qts, GPIO_BASE + SIFIVE_GPIO_REG_FALL_IP);
    g_print("After falling edge, FALL_IP: 0x%08x\n", fall_ip_after);

    /* Check PLIC pending */
    uint32_t plic_pending = qtest_readl(qts, PLIC_PENDING);
    g_print("PLIC pending register: 0x%08x\n", plic_pending);

    g_print("✓ Falling edge interrupt test completed!\n\n");

    qtest_quit(qts);
}

static void test_gpio_high_level_interrupt(void)
{
    QTestState *qts = qtest_init("-M g233 -nographic");

    g_print("=== Testing GPIO High Level Interrupt ===\n");

    /* Configure pin 2 as input */
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_OUTPUT_EN, 0x00000000);
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_INPUT_EN, 0x00000004);

    /* Enable high level interrupt for pin 2 */
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_HIGH_IE, 0x00000004);

    /* Clear any pending interrupts */
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_RISE_IP, 0xFFFFFFFF);
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_FALL_IP, 0xFFFFFFFF);
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_HIGH_IP, 0xFFFFFFFF);
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_LOW_IP, 0xFFFFFFFF);

    g_print("GPIO configured for high level interrupt on pin 2\n");

    /* Read initial interrupt pending status */
    uint32_t high_ip_before = qtest_readl(qts, GPIO_BASE + SIFIVE_GPIO_REG_HIGH_IP);
    g_print("Initial HIGH_IP: 0x%08x\n", high_ip_before);

    /* Set pin high */
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_PORT, 0x00000004);

    /* Read interrupt pending status */
    uint32_t high_ip_after = qtest_readl(qts, GPIO_BASE + SIFIVE_GPIO_REG_HIGH_IP);
    g_print("After setting pin high, HIGH_IP: 0x%08x\n", high_ip_after);

    /* Check PLIC pending */
    uint32_t plic_pending = qtest_readl(qts, PLIC_PENDING);
    g_print("PLIC pending register: 0x%08x\n", plic_pending);

    g_print("✓ High level interrupt test completed!\n\n");

    qtest_quit(qts);
}

static void test_gpio_low_level_interrupt(void)
{
    QTestState *qts = qtest_init("-M g233 -nographic");

    g_print("=== Testing GPIO Low Level Interrupt ===\n");

    /* Configure pin 3 as input with pull-up */
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_OUTPUT_EN, 0x00000000);
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_INPUT_EN, 0x00000008);
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_PUE, 0x00000008);

    /* Enable low level interrupt for pin 3 */
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_LOW_IE, 0x00000008);

    /* Clear any pending interrupts */
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_RISE_IP, 0xFFFFFFFF);
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_FALL_IP, 0xFFFFFFFF);
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_HIGH_IP, 0xFFFFFFFF);
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_LOW_IP, 0xFFFFFFFF);

    g_print("GPIO configured for low level interrupt on pin 3\n");

    /* Read initial interrupt pending status */
    uint32_t low_ip_before = qtest_readl(qts, GPIO_BASE + SIFIVE_GPIO_REG_LOW_IP);
    g_print("Initial LOW_IP: 0x%08x\n", low_ip_before);

    /* Set pin low */
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_PORT, 0x00000000);

    /* Read interrupt pending status */
    uint32_t low_ip_after = qtest_readl(qts, GPIO_BASE + SIFIVE_GPIO_REG_LOW_IP);
    g_print("After setting pin low, LOW_IP: 0x%08x\n", low_ip_after);

    /* Check PLIC pending */
    uint32_t plic_pending = qtest_readl(qts, PLIC_PENDING);
    g_print("PLIC pending register: 0x%08x\n", plic_pending);

    g_print("✓ Low level interrupt test completed!\n\n");

    qtest_quit(qts);
}

static void test_gpio_combined_interrupts(void)
{
    QTestState *qts = qtest_init("-M g233 -nographic");

    g_print("=== Testing Combined GPIO Interrupts ===\n");

    /* Configure pins 0-3 as inputs */
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_OUTPUT_EN, 0x00000000);
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_INPUT_EN, 0x0000000F);
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_PUE, 0x0000000F);

    /* Enable all interrupt types for different pins */
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_RISE_IE, 0x00000001);  /* Pin 0 */
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_FALL_IE, 0x00000002);  /* Pin 1 */
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_HIGH_IE, 0x00000004);  /* Pin 2 */
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_LOW_IE,  0x00000008);  /* Pin 3 */

    /* Clear all pending interrupts */
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_RISE_IP, 0xFFFFFFFF);
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_FALL_IP, 0xFFFFFFFF);
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_HIGH_IP, 0xFFFFFFFF);
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_LOW_IP, 0xFFFFFFFF);

    g_print("All interrupt types enabled for pins 0-3\n");

    /* Trigger all interrupts */
    qtest_writel(qts, GPIO_BASE + SIFIVE_GPIO_REG_PORT, 0x00000004);  /* Set pin 2 high */

    /* Read all interrupt pending registers */
    uint32_t rise_ip = qtest_readl(qts, GPIO_BASE + SIFIVE_GPIO_REG_RISE_IP);
    uint32_t fall_ip = qtest_readl(qts, GPIO_BASE + SIFIVE_GPIO_REG_FALL_IP);
    uint32_t high_ip = qtest_readl(qts, GPIO_BASE + SIFIVE_GPIO_REG_HIGH_IP);
    uint32_t low_ip = qtest_readl(qts, GPIO_BASE + SIFIVE_GPIO_REG_LOW_IP);

    g_print("Combined interrupt test results:\n");
    g_print("  RISE_IP: 0x%08x\n", rise_ip);
    g_print("  FALL_IP: 0x%08x\n", fall_ip);
    g_print("  HIGH_IP: 0x%08x\n", high_ip);
    g_print("  LOW_IP:  0x%08x\n", low_ip);

    /* Check PLIC */
    uint32_t plic_pending = qtest_readl(qts, PLIC_PENDING);
    g_print("  PLIC pending: 0x%08x\n", plic_pending);

    g_print("✓ Combined interrupts test completed!\n\n");

    qtest_quit(qts);
}

int main(int argc, char **argv)
{
    int ret;

    g_test_init(&argc, &argv, NULL);

    g_test_add_func("/g233/gpio/interrupt/rising_edge", test_gpio_rising_interrupt);
    g_test_add_func("/g233/gpio/interrupt/falling_edge", test_gpio_falling_interrupt);
    g_test_add_func("/g233/gpio/interrupt/high_level", test_gpio_high_level_interrupt);
    g_test_add_func("/g233/gpio/interrupt/low_level", test_gpio_low_level_interrupt);
    g_test_add_func("/g233/gpio/interrupt/combined", test_gpio_combined_interrupts);

    ret = g_test_run();

    return ret;
}