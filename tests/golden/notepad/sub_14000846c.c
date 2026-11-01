// L1 structure recovery (7 blocks)
void sub_14000846c_l1(void) {
    eax = dword_ADDR;
    esi = r8d;
    ebx = edx;
    rdi = rcx;
    if (eax != 0) {
        AcquireSRWLockExclusive(off_140034c40);
        eax = dword_ADDR;
        if (esi == 0) {
            ebx = -(ebx);
            eax = (((ebx != 0) ? 0xADDR : 0) + 0xADDR);
            *(u32*)(rdi) = (*(u32*)(rdi) & eax);
        } else if (esi != eax) {
            ebx = -(ebx);
            eax = (((ebx != 0) ? 0xADDR : 0) + 0xADDR);
            *(u32*)(rdi) = (*(u32*)(rdi) & eax);
            ReleaseSRWLockExclusive(off_140034c40);
            return;
        }
        var_24 = 0;
        var_20 = ebx;
        var_28 = rdi;
        sub_14000925c(off_140034c78, &var_20, 0xADDR);
        if (al == 0) {
            ebx = -(ebx);
            eax = (((ebx != 0) ? 0xADDR : 0) + 0xADDR);
            *(u32*)(rdi) = (*(u32*)(rdi) & eax);
            ReleaseSRWLockExclusive(off_140034c40);
            return;
        }
        ReleaseSRWLockExclusive(off_140034c40);
    }
    return;
}
