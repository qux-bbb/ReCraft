// L1 structure recovery (3 blocks)
void sub_140025470_l1(void) {
    rdi = rcx;
    ebx = edx;
    rcx = *(u64*)((rcx + 0xADDR));
    WindowsDeleteString(*(u64*)((rcx + 0xADDR)));
    *(u64*)((rdi + 0xADDR)) = 0;
    *(u32*)((rdi + 0xADDR)) = 0xADDR;
    if ((bl & 1) != 0) {
        rcx = rdi;
        sub_1400025e0(rdi);
    }
    rax = rdi;
    return;
}
