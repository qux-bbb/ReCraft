// L1 structure recovery (3 blocks)
void libc_probe_l1(void) {
    malloc(0xADDR);
    if (rax == 0) {
        eax = -1;
        return;
    }
    *(u32*)(rax) = 0xADDR;
    rbx = rax;
    puts(rax);
    free(rbx);
    eax = 3;
    return;
}
