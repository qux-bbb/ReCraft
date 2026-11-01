// L1 structure recovery (3 blocks)
void sub_140024d58_l1(void) {
    rbp = *(u64*)(rcx);
    edi = 0;
    rbx = rdx;
    rsi = rcx;
    if (rbp != 0) {
        GetProcessHeap();
        HeapFree(rax, 0, rbp);
    }
    rax = rsi;
    rdi = ((rbx != 0) ? rbx : rdi);
    *(u64*)(rsi) = rdi;
    return;
}
