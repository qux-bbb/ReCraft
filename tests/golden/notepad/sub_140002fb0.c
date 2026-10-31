// L1 structure recovery (6 blocks)
void sub_140002fb0_l1(void) {
    rbx = rdx;
    if (*(u8*)(rdx) != 0) {
        rsi = (rdx + 0xADDR);
        AcquireSRWLockExclusive(rsi);
        rdi = (rbx + 0xADDR);
        rax = *(u64*)((rdi + 8));
        rax -= *(u64*)(rdi);
        if (rax >= 0xADDR) {
            sub_140009508(rdi);
            *(u64*)((rdi + 8)) = *(u64*)(rdi);
        }
        *(u8*)((rbx + 0xADDR)) = 0;
        if (rsi != 0) {
            ReleaseSRWLockExclusive(rsi);
        }
    }
    return;
}
