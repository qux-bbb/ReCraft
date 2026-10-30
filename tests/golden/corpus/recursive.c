// L1 structure recovery (3 blocks)
void recursive_l1(void) {
    ebx = ecx;
    if (ecx > 1 /* signed */) {
        ecx -= 1;
        recursive(ecx);
        eax *= ebx;
        return;
    }
    eax = 1;
    return;
}
