// L1 structure recovery (9 blocks)
void sub_140004890_l1(void) {
    rbx = rcx;
    rbp = rdx;
    eax = *(u32*)((rcx + 0xADDR));
    al &= 3;
    if (al != 2) {
        eax = 0;
    } else if ((cl & 0xADDR) == 0) {
        eax = 0;
        return;
    }
    edi = *(u32*)(rbx);
    edi -= *(u32*)((rbx + 8));
    *(u32*)((rbx + 0xADDR)) = 0;
    rsi = *(u64*)((rbx + 8));
    *(u64*)(rbx) = rsi;
    if (edi <= 0 /* signed */) {
        eax = 0;
        return;
    }
    sub_140009a28(rbx);
    sub_14000a27c(eax, rsi, edi, rbp);
    if (edi == eax) {
        eax = (*(u32*)((rbx + 0xADDR)) >> 2);
        if ((al & 1) == 0) {
            eax = 0;
            return;
        }
        *(u32*)((rbx + 0xADDR)) &= -0xADDR;
    } else {
        *(u32*)((rbx + 0xADDR)) |= 0xADDR;
        eax = -1;
    }
    return;
}
