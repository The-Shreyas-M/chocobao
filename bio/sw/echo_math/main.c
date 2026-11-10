#include <stdint.h>

#include "bio.h" // this must always be first

// First BIO test, needs NO wiring: the host writes a number into FIFO0
// (console: `bio tx <n>`), this core answers with 3*n + 7 (`bio rx`).
// BIO has no multiply instruction, so 3*n is (n << 1) + n.
void main(void) {
    while (1) {
        uint32_t x = pop_fifo0(); // blocks until the host sends a value
        push_fifo0((x << 1) + x + 7);
    }
}
