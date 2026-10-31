// L1 structure recovery (9 blocks)
void sub_140006f30_l1(void) {
    eax = (uint32)(*(u8*)(rdx));
    xmm0 = 0;
    r8 = rcx;
    var_0 = xmm0;
    var_10 = xmm0;
    if (al != 0) {
        do {
            eax = (uint32)(al);
            var_0 |= (1 << (eax & 0xADDR));
            eax = (uint32)(*(u8*)((rdx + 1)));
            rdx += 1;
        } while (al != 0);
    }
    ecx = (uint32)(*(u8*)(r8));
    if (cl == 0) {
        eax = 0;
        return;
    }
    while (1) {
        eax = (uint32)(cl);
        cl &= 7;
        rax >>= 3;
        edx = (uint32)(*(u8*)((rsp + rax)));
        dl >>= cl;
        if ((dl & 1) != 0) break;
        ecx = (uint32)(*(u8*)((r8 + 1)));
        r8 += 1;
        if (cl != 0) continue;
    }
    rax = r8;
    return;
}
