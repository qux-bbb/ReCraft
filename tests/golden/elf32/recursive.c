// L1 structure recovery (4 blocks)
void recursive_l1(void) {
    eax = var_4;
    edx = 1;
    if (eax > 1 /* signed */) {
        do {
            ecx = eax;
            eax -= 1;
            edx *= ecx;
        } while (eax != 1);
    }
    eax = edx;
    return;
}
