// L1 structure recovery (8 blocks)
void sub_14000a1b0_l1(void) {
    rbx = rcx;
    edi = edx;
    rcx = *(u64*)((rcx + 0xADDR));
    if (rcx == 0) {
        *(u32*)((rbx + 0xADDR)) = 0xADDR;
        rcx = *(u64*)((rbx + 0xADDR));
        if (rcx != 0) {
            *(u64*)((rbx + 0xADDR)) = 0;
            rax = *(u64*)(rcx);
            rax = *(u64*)((rax + 0xADDR));
            sub_140028010();
        }
        if ((dil & 1) != 0) {
            sub_140001c84(rbx, 0xADDR);
        }
        rax = rbx;
        return;
    }
    CloseHandle(rcx);
    if (eax == 0) {
        sub_1400091e4(var_28, 0xADDR);
    }
}
