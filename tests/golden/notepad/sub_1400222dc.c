// L1 structure recovery (13 blocks)
void sub_1400222dc_l1(void) {
    rax = -1;
    rsi = (rdx + 0xADDR);
    ebp = 0;
    rbx = rcx;
    do {
        rax += 1;
    } while (*(u16*)((rsi + (rax * 2))) != bp);
    if (rax == 0xADDR) {
        edi = 0xADDR;
        edx = 0;
        sub_140009e44(&var_38, 0, edi);
        rax = var_38;
        rdx = rax;
        rsi -= rax;
        do {
            rcx = (rdi + 0xADDR);
            if (rcx == 0) break;
            ecx = (uint32)(*(u16*)((rsi + rdx)));
            if (cx == 0) break;
            *(u16*)(rdx) = cx;
            rdx += 2;
            rdi -= 1;
        } while (rdi != 0);
        rcx = (rdx - 2);
        rcx = ((rdi != 0) ? rdx : rcx);
        *(u16*)(rcx) = bp;
        if (rdi != 0) {
            *(u64*)(rbx) = rax;
        } else {
            *(u64*)(rbx) = rbp;
            if (rax != 0) {
                CoTaskMemFree(rax);
            }
        }
    } else {
        *(u64*)(rcx) = rbp;
    }
    rax = rbx;
    return;
}
