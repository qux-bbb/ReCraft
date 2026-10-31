// L1 structure recovery (7 blocks)
void sub_1400040c8_l1(void) {
    rdi = *(u64*)((rcx + 0xADDR));
    rbx = rcx;
    *(u64*)((rcx + 0xADDR)) = 0;
    if (rdi != 0) {
        GetProcessHeap();
        HeapFree(rax, 0, rdi);
    }
    rdi = *(u64*)((rbx + 0xADDR));
    *(u64*)((rbx + 0xADDR)) = 0;
    if (rdi != 0) {
        GetProcessHeap();
        HeapFree(rax, 0, rdi);
    }
    rdi = *(u64*)((rbx + 0xADDR));
    *(u64*)((rbx + 0xADDR)) = 0;
    if (rdi != 0) {
        GetProcessHeap();
        HeapFree(rax, 0, rdi);
    }
    return;
}
