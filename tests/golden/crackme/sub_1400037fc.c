// L1 structure recovery (29 blocks)
void sub_1400037fc_l1(void) {
    rbx = (int64)(*(u32*)((rcx + 4)));
    edi = 0;
    r15 = r8;
    r14 = rdx;
    rsi = rcx;
    if (ebx == 0) {
    } else {
        sub_140002b30(rcx);
        r9 = 1;
        r9 += rbx;
        if (r9 == 0) {
            eax = 1;
            return;
        }
        rbx = (int64)(*(u32*)((rsi + 4)));
        if (ebx == 0) {
            rcx = rdi;
        } else {
            sub_140002b30(rcx);
            rcx = (rbx + rax);
        }
        if (*(u8*)((rcx + 0xADDR)) == dil) {
            eax = 1;
            return;
        } else if ((*(u8*)(rsi) & 0xADDR) == 0) {
            rbp = (int64)(*(u32*)((rsi + 4)));
            if (ebp == 0) {
                rbx = rdi;
            } else {
                sub_140002b30(rcx);
                rbx = (rax + rbp);
            }
            sub_140002b44(rcx);
            rcx = (int64)(*(u32*)((r14 + 4)));
            rcx += rax;
            if (rbx == rcx) {
                al = 2;
                if ((*(u8*)(r14) & 2) == 0 && (*(u8*)(r15) & 1) == 0 && (*(u8*)(r15) & 4) == 0 && (*(u8*)(r15) & 2) == 0) {
                    edi = 1;
                } else if ((*(u8*)(rsi) & al) != 0) {
                    edi = 1;
                    eax = 1;
                    return;
                }
                eax = edi;
            } else {
                rbx = (int64)(*(u32*)((rsi + 4)));
                if (ebx == 0) {
                    rbp = rdi;
                } else {
                    sub_140002b30(rcx);
                    rbp = (rbx + rax);
                }
                rbx = (int64)(*(u32*)((r14 + 4)));
                sub_140002b44(rcx);
                rdx = (rbx + 0xADDR);
                rdx += rax;
                sub_140016b60((rbp + 0xADDR), rdx);
                if (eax != 0) {
                    eax = 0;
                }
            }
        } else if ((*(u8*)(r14) & 0xADDR) != 0) {
            eax = 1;
            return;
        }
    }
    return;
}
