// L1 structure recovery (5 blocks)
void sub_140012540_l1(void) {
    eax = dword_ADDR;
    if (eax != 0) {
        AcquireSRWLockExclusive(off_140034c40);
        eax = dword_ADDR;
        if (eax != 0) {
            sub_140006b2c(off_140034c38);
            byte_ADDR = 0;
        }
        ReleaseSRWLockExclusive(off_140034c40);
    }
    return;
}
