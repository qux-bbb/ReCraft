// L1 structure recovery (13 blocks)
void sub_140004d94_l1(void) {
    ebx = dword_ADDR;
    if (ebx == 0) {
        eax = dword_ADDR;
        if (eax == 0) {
            ebx = 0;
        } else {
            AcquireSRWLockExclusive(off_140034c40);
            if (qword_ADDR == 0) {
                qword_ADDR = 0;
                rax = qword_ADDR;
                if (rax != 0) {
                    sub_140028010(off_140034c98, 0xADDR, off_140034c38);
                } else {
                    rax = qword_ADDR;
                }
                rcx = off_140034c40;
                if (qword_ADDR != 0) {
                    dword_ADDR = 1;
                    ReleaseSRWLockExclusive(off_140034c40);
                } else {
                    ReleaseSRWLockExclusive(off_140034c40);
                }
            } else {
                ebx = dword_ADDR;
                rcx = off_140034c40;
            }
        }
    }
    eax = ebx;
    return;
}
