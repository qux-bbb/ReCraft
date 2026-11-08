// L1 structure recovery (3 blocks)
void libc_probe_l1(void) {
    edi = 0xADDR;
    sub_1110();
    if (rax == 0) {
        eax = -1;
        return;
    }
    *(u32*)(rax) = 0xADDR;
    rbx = rax;
    rdi = rax;
    sub_10f0();
    rdi = rbx;
    sub_10d0();
    eax = 3;
    return;
}
