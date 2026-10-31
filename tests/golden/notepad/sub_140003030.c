// L1 structure recovery (9 blocks)
void sub_140003030_l1(void) {
    rbx = rdx;
    if (*(u8*)(rdx) != 0) {
        rdi = (rdx + 0xADDR);
        AcquireSRWLockExclusive(rdi);
        *(u8*)((rbx + 0xADDR)) = 0;
        if (rdi != 0) {
            ReleaseSRWLockExclusive(rdi);
        }
        if (byte_ADDR == 0) {
            if (qword_ADDR == 0) {
                sub_140004cb4(rbx);
                if (al != 0) {
                    rdx = *(u64*)((rbx + 0xADDR));
                    sub_14000647c((rdx + 0xADDR), *(u64*)((rbx + 0xADDR)));
                    sub_140006f64(*(u64*)((rbx + 0xADDR)));
                }
            } else {
                sub_140028010();
            }
        }
    }
    return;
}
