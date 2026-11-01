// L1 structure recovery (3 blocks)
void sub_140024c8c_l1(void) {
    rdi = *(u64*)(rcx);
    rbx = rcx;
    if (rdi != 0) {
        GetProcessHeap();
        HeapFree(rax, 0, rdi);
        *(u64*)(rbx) = 0;
    }
    return;
}
