// L1 structure recovery (12 blocks)
void sub_140004608_l1(void) {
    rdi = r9;
    rbx = r8;
    sub_1400045f0(*(u64*)(rdx));
    rdx = *(u64*)((rbx + 8));
    rax = *(u64*)(rbx);
    rcx = *(u64*)(rax);
    if (rcx != 0) {
        ecx = *(u32*)((rcx + 0xADDR));
        eax = ecx;
        eax >>= 0xADDR;
        if ((al & 1) != 0) {
            eax = ecx;
            al &= 3;
            if (al != 2) {
                if (((ecx >> 0xADDR) & 1) != 0) {
                    rax = *(u64*)((rbx + 0xADDR));
                    if (*(u8*)(rax) != 0) {
                        rcx = *(u64*)(rbx);
                        rcx = *(u64*)(rcx);
                        sub_14000491c(rcx);
                        if (eax == -1) {
                            rax = *(u64*)((rbx + 0xADDR));
                            *(u32*)(rax) = (*(u32*)(rax) | -1);
                        } else {
                            rax = *(u64*)((rbx + 8));
                            *(u32*)(rax) = (*(u32*)(rax) + 1);
                        }
                    } else {
                        rax = *(u64*)(rbx);
                        rcx = *(u64*)(rax);
                        eax = *(u32*)((rcx + 0xADDR));
                        eax >>= 1;
                    }
                } else {
                    *(u32*)(rdx) = (*(u32*)(rdx) + 1);
                }
            }
        }
    }
    sub_1400045fc(*(u64*)(rdi));
    return;
}
