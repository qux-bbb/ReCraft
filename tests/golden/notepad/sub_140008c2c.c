// L1 structure recovery (9 blocks)
void sub_140008c2c_l1(void) {
    if (r8 != 0) {
        rsi = r8;
        rdi = rdx;
        rbx = rcx;
        EnterCriticalSection(rcx);
        AcquireSRWLockExclusive(rdi);
        rdx = *(u64*)((rbx + 0xADDR));
        rcx = (rsi - 1);
        rax = *(u64*)((rbx + 0xADDR));
        rax -= rdx;
        rax >>= 4;
        if (rcx < rax) {
            rcx += rcx;
            *(__m128*)((rdx + (rcx * 8))) = 0;
        }
        if (rdi != 0) {
            ReleaseSRWLockExclusive(rdi);
        }
        if (rbx != 0) {
            LeaveCriticalSection(rbx);
        }
    }
    return;
}
