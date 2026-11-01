// L1 structure recovery (6 blocks)
void sub_14000925c_l1(void) {
    rbx = rcx;
    rbp = r8;
    rcx = *(u64*)((rcx + 0xADDR));
    r14 = rdx;
    rcx -= *(u64*)(rbx);
    rsi = (rbx + 8);
    rax = *(u64*)(rsi);
    rax -= *(u64*)(rbx);
    rax += r8;
    if (rax >= rcx) {
        rax = (rcx + rcx);
        rdx = rbp;
        rdx = ((rbp < rax) ? rax : rdx);
        sub_140009300(rbx, rdx);
        if (al != 0) {
            rdi = (rbx + 8);
            rcx = *(u64*)(rsi);
            r9 = rbp;
            rax = *(u64*)((rbx + 0xADDR));
            rax -= rcx;
            rdx = ((rcx < *(u64*)((rbx + 0xADDR))) ? rax : 0);
            sub_1400096fc(*(u64*)(rsi), rdx, r14);
            *(u64*)(rdi) = (*(u64*)(rdi) + rbp);
            al = 1;
        }
    } else {
        rdi = rsi;
        rcx = *(u64*)(rsi);
        r9 = rbp;
        rax = *(u64*)((rbx + 0xADDR));
        rax -= rcx;
        rdx = ((rcx < *(u64*)((rbx + 0xADDR))) ? rax : 0);
        sub_1400096fc(*(u64*)(rsi), rdx, r14);
        *(u64*)(rdi) = (*(u64*)(rdi) + rbp);
        al = 1;
        return;
    }
    return;
}
