// L1 structure recovery (8 blocks)
void sub_140008748_l1(void) {
    *(u64*)(rdx) = 0;
    rbx = (rcx + 0xADDR);
    rcx = *(u64*)(rbx);
    eax = 0;
    r10 = *(u64*)((rbx + 8));
    r11 = r8;
    r10 -= rcx;
    rdi = rdx;
    r10 >>= 4;
    if (r10 == 0) {
        var_20 = r11;
        var_28 = r9;
        sub_14000925c(rbx, &var_20, 0xADDR);
        if (al != 0) {
            rax = *(u64*)((rbx + 8));
            rax -= *(u64*)(rbx);
            rax >>= 4;
            *(u64*)(rdi) = rax;
        }
    } else {
        rdx = rcx;
        while (1) {
            r8 = (rax + 1);
            if (*(u64*)(rdx) == 0) break;
            rdx += 0xADDR;
            rax = r8;
            if (r8 < r10) continue;
        }
        rax += rax;
        *(u64*)((rcx + (rax * 8))) = r11;
        *(u64*)(((rcx + (rax * 8)) + 8)) = r9;
        *(u64*)(rdi) = r8;
    }
    return;
}
