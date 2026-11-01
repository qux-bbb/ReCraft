// L1 structure recovery (5 blocks)
void sub_140023648_l1(void) {
    ebx = ecx;
    rdi = off_140036178;
    CoCreateGuid(rdi);
    qword_ADDR = 0;
    if (dword_ADDR > 4) {
        rdx = 0xADDR;
        if ((qword_ADDR & 0xADDR) != 0) {
            rax = qword_ADDR;
            rax &= rdx;
            if (rax == qword_ADDR) {
                eax = 1;
                var_58 = ebx;
                qword_ADDR = rax;
                rdx = off_14002e964;
                var_60 = rax;
                var_30 = &var_58;
                var_28 = &var_60;
                rax = &var_68;
                var_20 = rax;
                var_68 = rdi;
                sub_1400011bc(rcx);
            }
        }
    }
    return;
}
