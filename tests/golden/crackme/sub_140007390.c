// L1 structure recovery (38 blocks)
void sub_140007390_l1(void) {
    rsi = rdx;
    edi = ecx;
    sub_14000ab5c();
    rbx = rax;
    if (rax == 0) {
    } else {
        rcx = *(u64*)(0);
        rax = rcx;
        r8 = (rcx + 0xADDR);
        if (rcx == r8) {
            eax = 0;
            return;
        }
        while (*(u32*)(rax) != edi) {
            rax += 0xADDR;
            if (rax != r8) continue;
        }
        if (rax == 0) {
            eax = 0;
            return;
        }
        r8 = *(u64*)((rax + 8));
        if (r8 == 0) {
            eax = 0;
            return;
        } else if (r8 != 5) {
            if (r8 != 1) {
                rbp = *(u64*)((rbx + 8));
                *(u64*)((rbx + 8)) = rsi;
                if (*(u32*)((rax + 4)) != 8) {
                    ecx = *(u32*)((rax + 4));
                    *(u64*)((rax + 8)) = 0;
                    rax = r8;
                    sub_1400160e0(*(u32*)((rax + 4)));
                } else {
                    rcx += 0xADDR;
                    rdx = (rcx + 0xADDR);
                    while (rcx != rdx) {
                        *(u64*)((rcx + 8)) = 0;
                        rcx += 0xADDR;
                    }
                    edi = *(u32*)((rbx + 0xADDR));
                    if (*(u32*)(rax) == 0xADDR) {
                        *(u32*)((rbx + 0xADDR)) = 0xADDR;
                    } else if (*(u32*)(rax) == 0xADDR) {
                        *(u32*)((rbx + 0xADDR)) = 0xADDR;
                        rax = r8;
                        sub_1400160e0(8);
                        *(u32*)((rbx + 0xADDR)) = edi;
                        *(u64*)((rbx + 8)) = rbp;
                        eax = -1;
                        return;
                    } else if (*(u32*)(rax) == 0xADDR) {
                        *(u32*)((rbx + 0xADDR)) = 0xADDR;
                        rax = r8;
                        sub_1400160e0(8);
                        *(u32*)((rbx + 0xADDR)) = edi;
                        *(u64*)((rbx + 8)) = rbp;
                        eax = -1;
                        return;
                    } else if (*(u32*)(rax) == 0xADDR) {
                        *(u32*)((rbx + 0xADDR)) = 0xADDR;
                        rax = r8;
                        sub_1400160e0(8);
                        *(u32*)((rbx + 0xADDR)) = edi;
                        *(u64*)((rbx + 8)) = rbp;
                        eax = -1;
                        return;
                    } else if (*(u32*)(rax) == 0xADDR) {
                        *(u32*)((rbx + 0xADDR)) = 0xADDR;
                        rax = r8;
                        sub_1400160e0(8);
                        *(u32*)((rbx + 0xADDR)) = edi;
                        *(u64*)((rbx + 8)) = rbp;
                        eax = -1;
                        return;
                    } else if (*(u32*)(rax) == 0xADDR) {
                        *(u32*)((rbx + 0xADDR)) = 0xADDR;
                        rax = r8;
                        sub_1400160e0(8);
                        *(u32*)((rbx + 0xADDR)) = edi;
                        *(u64*)((rbx + 8)) = rbp;
                        eax = -1;
                        return;
                    } else if (*(u32*)(rax) == 0xADDR) {
                        *(u32*)((rbx + 0xADDR)) = 0xADDR;
                        rax = r8;
                        sub_1400160e0(8);
                        *(u32*)((rbx + 0xADDR)) = edi;
                        *(u64*)((rbx + 8)) = rbp;
                        eax = -1;
                        return;
                    } else if (*(u32*)(rax) == 0xADDR) {
                        *(u32*)((rbx + 0xADDR)) = 0xADDR;
                        rax = r8;
                        sub_1400160e0(8);
                        *(u32*)((rbx + 0xADDR)) = edi;
                        *(u64*)((rbx + 8)) = rbp;
                        eax = -1;
                        return;
                    }
                    edx = edi;
                    if (*(u32*)(rax) == 0xADDR) {
                        *(u32*)((rbx + 0xADDR)) = 0xADDR;
                        rax = r8;
                        sub_1400160e0(8);
                        *(u32*)((rbx + 0xADDR)) = edi;
                        *(u64*)((rbx + 8)) = rbp;
                        eax = -1;
                        return;
                    }
                    rax = r8;
                    sub_1400160e0(8);
                    *(u32*)((rbx + 0xADDR)) = edi;
                }
                *(u64*)((rbx + 8)) = rbp;
            }
            eax = -1;
        } else {
            *(u64*)((rax + 8)) = 0;
            eax = (r8 - 4);
        }
    }
    return;
}
