// L1 structure recovery (4 blocks)
void loopy_l1(void) {
    r10d = 0;
    edx = r10d;
    r8d = r10d;
    eax = r10d;
    if (ecx >= 2 /* signed */) {
        r9d = (rcx - 1);
        do {
            r8d += 1;
            edx += eax;
            r8d += eax;
            eax += 2;
        } while (eax < r9d /* signed */);
    }
    eax = ((eax >= ecx /* signed */) ? r10d : eax);
    eax += r8d;
    eax += edx;
    return;
}
