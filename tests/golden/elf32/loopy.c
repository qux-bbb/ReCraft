// L1 structure recovery (7 blocks)
void loopy_l1(void) {
    var_0 = ebx;
    ebx = var_8;
    if (ebx <= 0 /* signed */) {
        ebx = var_0;
        eax = 0;
        return;
    }
    ecx = (ebx + (ebx * 2));
    eax = 0;
    edx = 0;
    ebx &= 1;
    if (ebx == 0) {
        do {
            edx = ((edx + (eax * 2)) + 3);
            eax += 6;
        } while (eax != ecx);
    } else {
        eax = 3;
    }
    eax = edx;
    ebx = var_0;
    return;
}
