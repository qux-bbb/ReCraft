// L1 structure recovery (9 blocks)
void nested_loops_l1(void) {
    esi = var_10;
    if (esi <= 0 /* signed */) {
        ecx = 0;
    } else {
        ebx = 0;
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
        edi = ebx;
        edx = (ebx + eax);
        edi ^= eax;
        edx += ecx;
        edi &= 1;
        ecx = ((edi != 0) ? edx : ecx);
        edx = (eax + 1);
    } while (esi != edx);
    edx = (ebx + 1);
    if (ebx == eax) {
        eax = ecx;
        return;
    }
    ebx = edx;
    goto loc_ADDR;
}
