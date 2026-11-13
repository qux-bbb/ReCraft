// L1 structure recovery (20 blocks)
void sub_1400079a8_l1(void) {
    rsi = rcx;
    ecx = 0;
    r8 = rsi;
    dl = *(u8*)(rsi);
    while (dl != 0) {
        rax = (rcx + 1);
        rax = ((dl == 0xADDR) ? rcx : rax);
        rcx = rax;
        rax = -1;
        do {
            rax += 1;
        } while (*(u8*)((r8 + rax)) != 0);
        r8 += 1;
        r8 += rax;
        dl = *(u8*)(r8);
    }
    rcx += 1;
    sub_140008e60(rcx, 8);
    rbx = rax;
    if (rax != 0) {
        r14 = rbx;
        while (1) {
            al = *(u8*)(rsi);
            if (al == 0) break;
            rbp = -1;
            do {
                rbp += 1;
            } while (*(u8*)((rsi + rbp)) != 0);
            rbp += 1;
            if (al != 0xADDR) {
                sub_140008e60(rbp, 1);
                rdi = rax;
                strcpy_s(rax, rbp, rsi);
                ecx = 0;
                *(u64*)(r14) = rdi;
                r14 += 8;
                sub_140008ee0(0);
            }
            rsi += rbp;
        }
        sub_140008ee0(0);
        rax = rbx;
        return;
    }
    sub_140008ee0(0);
    eax = 0;
    return;
}
