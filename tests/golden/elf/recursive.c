// L1 structure recovery (4 blocks)
void recursive_l1(void) {
    eax = 1;
    if (edi > 1 /* signed */) {
        do {
            edx = edi;
            edi -= 1;
            eax *= edx;
        } while (edi != 1);
    }
    return;
}
