// L1 structure recovery (4 blocks)
void loopy_l1(void) {
    r10d = (r10d ^ r10d);
    edx = r10d;
    r8d = r10d;
    eax = r10d;
    if (!(ecx < 2 /* signed */)) {
        r9d = (rcx - 1);
        do {
            r8d = (r8d + 1);
            edx = (edx + eax);
            r8d = (r8d + eax);
            eax = (eax + 2);
        } while ((eax < r9d /* signed */));
    }
    eax = ((eax >= ecx /* signed */) ? r10d : eax);
    eax = (eax + r8d);
    eax = (eax + edx);
    return;
}
