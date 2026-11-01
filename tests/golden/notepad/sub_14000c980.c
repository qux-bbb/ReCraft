// L1 structure recovery (11 blocks)
void sub_14000c980_l1(void) {
    edi = 0;
    ebx = ecx;
    word_ADDR = di;
    if (ecx == 5) {
        rcx = qword_ADDR;
        dword_ADDR = 5;
        if (rcx != 0) {
            sub_14002708c(rcx);
            qword_ADDR = rdi;
        }
    } else {
        sub_140026b60();
        if (eax != 0) {
            eax = dword_ADDR;
            if (ebx != eax) {
                if (eax == 2) {
                    sub_1400268d4();
                    if (eax >= 0 /* signed */) {
                        rcx = qword_ADDR;
                        if (rcx != 0) {
                            sub_14002708c(rcx);
                            qword_ADDR = rdi;
                        }
                    }
                }
                dword_ADDR = ebx;
            }
        }
    }
    return;
}
