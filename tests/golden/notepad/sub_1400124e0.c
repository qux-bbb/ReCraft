// L1 structure recovery (6 blocks)
void sub_1400124e0_l1(void) {
    eax = *(u32*)(rdx);
    rdi = rdx;
    if (eax != 0) {
        rbx = (rdx + 8);
        AcquireSRWLockExclusive(rbx);
        eax = *(u32*)(rdi);
        if (eax != 0) {
            sub_140006b2c(rdi);
            *(u8*)((rdi + 0xADDR)) = 0;
        }
        if (rbx != 0) {
            ReleaseSRWLockExclusive(rbx);
        }
    }
    return;
}
