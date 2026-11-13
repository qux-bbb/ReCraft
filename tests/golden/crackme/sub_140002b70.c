// L1 structure recovery (1 blocks)
void sub_140002b70_l1(void) {
    rbx = rcx;
    __vcrt_getptd(rcx);
    *(u64*)((rax + 0xADDR)) = rbx;
    return;
}
