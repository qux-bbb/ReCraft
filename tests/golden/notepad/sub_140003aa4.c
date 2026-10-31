// L1 structure recovery (6 blocks)
void sub_140003aa4_l1(void) {
    rbp = (rcx + 0xADDR);
    rdi = rcx;
    if (rcx != rbp) {
        do {
            rsi = *(u64*)(rdi);
            while (rsi != 0) {
                rbx = rsi;
                rsi = *(u64*)((rsi + 8));
                GetProcessHeap();
                HeapFree(rax, 0, rbx);
            }
            *(u64*)(rdi) = rsi;
            rdi += 8;
        } while (rdi != rbp);
    }
    return;
}
