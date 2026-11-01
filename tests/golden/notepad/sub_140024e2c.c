// L1 structure recovery (1 blocks)
void sub_140024e2c_l1(void) {
    rbx = rcx;
    GetProcessHeap();
    r8 = rbx;
    HeapFree(rax, 0, rbx);
    return;
}
