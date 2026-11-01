// L1 structure recovery (18 blocks)
void sub_140009e44_l1(void) {
    r12d = 0;
    rsi = r8;
    rbp = rdx;
    r14 = rcx;
    if (rdx != 0) {
        rbx = rsi;
        if (rbp != 0) {
            ecx = 0xADDR;
            rax = rsi;
            rbx = rbp;
            rax = ((rsi >= rcx) ? rcx : rax);
            if (rax != 0) {
                while (*(u16*)(rbx) != r12w) {
                    rbx += 2;
                    rax -= 1;
                    if (rax == 0) break;
                }
            }
            rbx -= rbp;
            rbx >>= 1;
        }
        rsi = ((rsi == -1) ? rbx : rsi);
        r15 = ((rsi * 2) + 2);
        CoTaskMemAlloc(r15);
        rdi = rax;
        if (rax != 0) {
            if (rbp == 0) {
                *(u16*)(rax) = r12w;
            } else {
                rbx += rbx;
                if (rbx != 0) {
                    rcx = rax;
                    if (r15 < rbx) {
                        r8 = r15;
                        edx = 0;
                        sub_1400029dc();
                        _o__errno();
                        *(u32*)(rax) = 0xADDR;
                        sub_140002934();
                    } else {
                        r8 = rbx;
                        rdx = rbp;
                        sub_1400274ac();
                    }
                }
                *(u16*)((rbx + rdi)) = r12w;
            }
            *(u16*)((rdi + (rsi * 2))) = r12w;
        }
        rax = r14;
        *(u64*)(r14) = rdi;
        return;
    }
    if (r8 == -1) {
        sub_140009200(var_38, 0xADDR, "onecore\\internal\\sdk\\inc\\wil\\opensource\\wil\\resource.h");
    }
}
