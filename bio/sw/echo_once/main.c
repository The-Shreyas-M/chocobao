#include <stdint.h>

#include "bio.h" // this must always be first

// One-shot version: answer a single request, then idle (so we don't read our own answer back).
void main(void) {
    uint32_t x = pop_fifo0();        // blocks until the host sends a value
    push_fifo0((x << 1) + x + 7);    // 3*x + 7
    while (1) {
        wait_quantum();              // idle
    }
}
