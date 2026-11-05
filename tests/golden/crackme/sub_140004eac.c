// L1 structure recovery (10 blocks)
void sub_140004eac_l1(void) {
    rax = 0xADDR;
    rbp = rcx;
    if (rdx <= 0xADDR) {
        rsi = (rdx + rdx);
        if (*(u64*)((rcx + 0xADDR)) != 0) {
            if (rsi > *(u64*)((rcx + 0xADDR))) {
                sub_14000b080(rsi);
                rbx = rax;
                if (rax != 0) {
                    sub_140008ee0(*(u64*)((rbp + 0xADDR)));
                    *(u64*)((rbp + 0xADDR)) = rbx;
                    dil = 1;
                    *(u64*)((rbp + 0xADDR)) = rsi;
                }
                sub_140008ee0(0);
                al = dil;
            } else {
                al = 1;
            }
        } else if (rsi <= 0xADDR) {
            al = 1;
            return;
        }
    } else {
        *(u8*)((r8 + 0xADDR)) = 1;
        al = 0;
        *(u32*)((r8 + 0xADDR)) = 0xADDR;
    }
    return;
}
