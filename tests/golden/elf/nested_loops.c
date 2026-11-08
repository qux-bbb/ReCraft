// L1 structure recovery (9 blocks)
void nested_loops_l1(void) {
    if (edi <= 0 /* signed */) {
        ecx = 0;
    } else {
        esi = 0;
        ecx = 0;
        loc_ADDR:
        eax = 0;
        goto loc_ADDR;
    }
    eax = ecx;
    return;
    do {
        eax = edx;
        loc_ADDR:
        r8d = esi;
        edx = (rsi + rax);
        r8d ^= eax;
        edx += ecx;
        r8d &= 1;
        ecx = ((r8d != 0) ? edx : ecx);
        edx = (rax + 1);
    } while (edi != edx);
    edx = (rsi + 1);
    if (esi == eax) {
        eax = ecx;
        return;
    }
    esi = edx;
    goto loc_ADDR;
}
