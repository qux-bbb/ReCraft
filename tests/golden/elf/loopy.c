// L1 structure recovery (7 blocks)
void loopy_l1(void) {
    if (edi <= 0 /* signed */) {
        eax = 0;
        return;
    }
    ecx = (rdi + (rdi * 2));
    eax = 0;
    edx = 0;
    edi &= 1;
    if (edi == 0) {
        do {
            edx = ((rdx + (rax * 2)) + 3);
            eax += 6;
        } while (ecx != eax);
    } else {
        eax = 3;
    }
    eax = edx;
    return;
}
