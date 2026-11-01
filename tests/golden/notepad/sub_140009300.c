// L1 structure recovery (8 blocks)
void sub_140009300_l1(void) {
    rax = *(u64*)((rcx + 0xADDR));
    rdi = rdx;
    rax -= *(u64*)(rcx);
    rbx = rcx;
    if (rax >= rdx) {
        al = 1;
    } else {
        GetLastError();
        rdi &= -0xADDR;
        ecx = 0;
        rdi += 0xADDR;
        ebp = eax;
        sub_140006634(0, rdi);
        rsi = rax;
        if (rax != 0) {
            r14 = *(u64*)((rbx + 8));
            r14 -= *(u64*)(rbx);
            r9 = r14;
            sub_1400096fc(rsi, rdi, *(u64*)(rbx));
            r15 = *(u64*)((rbx + 0xADDR));
            *(u64*)((rbx + 0xADDR)) = rsi;
            if (r15 != 0) {
                GetProcessHeap();
                HeapFree(rax, 0, r15);
            }
            rax = (r14 + rsi);
            *(u64*)(rbx) = rsi;
            *(u64*)((rbx + 8)) = rax;
            rax = (rsi + rdi);
            *(u64*)((rbx + 0xADDR)) = rax;
            SetLastError(ebp);
        } else {
            SetLastError(ebp);
            al = 0;
        }
    }
    return;
}
