// L1 structure recovery (1 blocks)
void sub_140003b90_l1(void) {
    rax = "bad exception";
    *(u64*)((rcx + 0xADDR)) = 0;
    *(u64*)((rcx + 8)) = rax;
    *(u64*)(rcx) = 0xADDR;
    rax = rcx;
    return;
}
