// L1 structure recovery (15 blocks)
void sub_140007004_l1(void) {
    r13d = var_a8;
    r15 = r9;
    r14 = var_a0;
    rbp = r8;
    var_28 = r13d;
    r12 = rdx;
    var_20 = r14;
    rbx = rcx;
    sub_14000717c(rcx, rdx, r8, r9, var_a0, var_a8);
    if (al == 0) {
        r8 = (r14 + 0xADDR);
        r8 += rbp;
        rcx = (rbx + 0xADDR);
        if (*(u64*)(rcx) != 0) {
            if (*(u8*)((rbx + 0xADDR)) != 0) {
                rax = *(u64*)((rcx + 8));
                rax -= *(u64*)(rcx);
                rdx = *(u64*)((rcx + 0xADDR));
                rax += r8;
                rdx -= *(u64*)(rcx);
                if (rax >= rdx) {
                    rdx += rdx;
                    rdx = ((r8 >= rdx) ? r8 : rdx);
                    sub_140009300(rcx, rdx);
                }
            }
        } else {
            xmm0 = 0;
            rdx = (r14 + 0xADDR);
            xmm1 = 0;
            rdx += rbp;
            var_30 = xmm0;
            var_40 = xmm1;
            sub_140009300(&var_30, rdx);
            if (al == 0) {
                rdi = var_48;
            } else {
                rdx = var_30;
                r8d = 0;
                r9 = var_40;
                r9 -= rdx;
                sub_140008078(rbx, var_30, 0, r9);
                rsi = *(u64*)((rbx + 0xADDR));
                edi = 0;
                rax = var_48;
                *(u64*)((rbx + 0xADDR)) = rax;
                if (rsi != 0) {
                    GetProcessHeap();
                    HeapFree(rax, 0, rsi);
                }
                *(u8*)((rbx + 0xADDR)) = 1;
            }
            if (rdi != 0) {
                GetProcessHeap();
                HeapFree(rax, 0, rdi);
            }
        }
        var_28 = r13d;
        var_20 = r14;
        sub_14000717c(rbx, r12, rbp, r15, r14, r13d);
    } else {
        al = 1;
    }
    r11 = &var_50;
    return;
}
