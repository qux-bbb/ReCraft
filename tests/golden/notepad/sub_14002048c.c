// L1 structure recovery (12 blocks)
void sub_14002048c_l1(void) {
    rcx = qword_ADDR;
    if (rcx != 0) {
        GlobalLock(qword_ADDR);
        rcx = qword_ADDR;
        rdi = rax;
        eax = 0;
        if (rcx != 0) {
            GlobalLock(qword_ADDR);
        }
        ebx = 0;
        if (rdi != 0) {
            rdx = (rdi + (rcx * 2));
            ecx = (uint32)(*(u16*)(rdi));
            rcx = (rdi + (rcx * 2));
            CreateDCW((rdi + (rcx * 2)), (rdi + (rcx * 2)), 0, rax);
            rbx = rax;
        }
        GlobalUnlock(qword_ADDR);
        rcx = qword_ADDR;
        if (rcx != 0) {
            GlobalUnlock(qword_ADDR);
        }
        if (rbx == 0) {
            MessageBoxW(qword_ADDR, qword_ADDR, qword_ADDR, 0xADDR);
            eax = 0;
        } else {
            rax = rbx;
        }
    } else {
        dword_ADDR = (dword_ADDR | 0xADDR);
        sub_14000f5b8(off_140035550);
        rcx = qword_ADDR;
        dword_ADDR = (dword_ADDR & 0xADDR);
        if (rcx == 0) {
            MessageBoxW(qword_ADDR, qword_ADDR, qword_ADDR, 0xADDR);
            eax = 0;
            return;
        }
    }
    return;
}
