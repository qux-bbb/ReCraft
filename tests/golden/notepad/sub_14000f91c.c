// L1 structure recovery (11 blocks)
void sub_14000f91c_l1(void) {
    CommDlgExtendedError();
    ebx = 0;
    r9d = eax;
    if (eax != 0) {
        rdx = rbx;
        rcx = off_140034178;
        r8d = ebx;
        do {
            if (r9d == *(u32*)((rcx - 8))) {
                rdx = *(u64*)(*(u64*)(rcx));
            }
            r8d += 1;
            rcx += 0xADDR;
        } while (r8d < 8);
        if (rdx == 0) {
            sub_1400083f8(&var_20, 0xADDR, qword_ADDR, r9);
            rdx = &var_20;
        }
        if (*(u16*)(rdx) != bx) {
            MessageBoxW(qword_ADDR, rdx, qword_ADDR, 0xADDR);
        }
        eax = 1;
    }
    return;
}
