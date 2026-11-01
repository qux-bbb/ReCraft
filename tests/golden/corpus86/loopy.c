// L1 structure recovery (4 blocks)
void loopy_l1(void) {
    edi = var_c;
    ecx = 0;
    edx = 0;
    eax = 0;
    if (edi >= 2 /* signed */) {
        esi = (edi + 0xADDR);
        do {
            edx += 1;
            ecx += eax;
            edx += eax;
            eax += 2;
        } while (eax < esi /* signed */);
    }
    esi = 0;
    eax = ((eax >= edi /* signed */) ? esi : eax);
    eax += edx;
    eax += ecx;
    return;
}
