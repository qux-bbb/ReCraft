// L1 structure recovery (6 blocks)
void sub_140003b20_l1(void) {
    rbp = (rcx + 0xADDR);
    rdi = rcx;
    if (rcx != rbp) {
        do {
            rsi = *(u64*)(rdi);
            while (rsi != 0) {
                rbx = rsi;
                rsi = *(u64*)((rsi + 8));
                sub_140004008((rbx + 0xADDR));
                GetProcessHeap();
                HeapFree(rax, 0, rbx);
            }
            *(u64*)(rdi) = rsi;
            rdi += 8;
        } while (rdi != rbp);
    }
    return;
}
