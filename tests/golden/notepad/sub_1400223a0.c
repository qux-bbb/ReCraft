// L1 structure recovery (9 blocks)
void sub_1400223a0_l1(void) {
    rdi = rdx;
    rbx = rcx;
    sub_140009e44(&var_40, "RestartByRestartManager:", 0xADDR);
    rax = var_40;
    r9d = 0xADDR;
    r8 = (rax + 0xADDR);
    rdi -= r8;
    do {
        rcx = (r9 + 0xADDR);
        if (rcx == 0) break;
        ecx = (uint32)(*(u16*)((rdi + r8)));
        if (cx == 0) break;
        *(u16*)(r8) = cx;
        r8 += 2;
        r9 -= 1;
    } while (r9 != 0);
    rcx = (r8 - 2);
    rcx = ((r9 != 0) ? r8 : rcx);
    *(u16*)(rcx) = 0xADDR;
    if (r9 != 0) {
        *(u64*)(rbx) = rax;
    } else {
        *(u64*)(rbx) = 0;
        if (rax != 0) {
            CoTaskMemFree(rax);
        }
    }
    rax = rbx;
    return;
}
