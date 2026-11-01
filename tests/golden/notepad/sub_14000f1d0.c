// L1 structure recovery (20 blocks)
void sub_14000f1d0_l1(void) {
    if (edx == 0xADDR) {
        if (*(u32*)((r9 + 0xADDR)) == 0xADDR) {
            r8d = 0;
            edx = 0xADDR;
            var_20 = r8;
            SendDlgItemMessageW(rcx, 0xADDR, (rdx + 0xADDR), 0, 0);
            eax += 1;
            dword_ADDR = eax;
        }
    } else if (edx == 0xADDR) {
        edx = dword_ADDR;
        r8d = 0;
        if (edx == 0) {
            edx = dword_ADDR;
        }
        eax = edx;
        dword_ADDR = edx;
        if (eax != 6) {
            edx -= 1;
            if (edx == 0) {
                r8 = qword_ADDR;
            } else {
                edx -= 1;
                if (edx == 0) {
                    r8 = qword_ADDR;
                } else {
                    edx -= 1;
                    if (edx == 0) {
                        r8 = qword_ADDR;
                    } else {
                        edx -= 1;
                        if (edx == 0) {
                            r8 = qword_ADDR;
                        } else if (edx == 1) {
                            r8 = qword_ADDR;
                        }
                    }
                }
            }
        } else {
            r8 = "GB18030";
        }
        edx = 0;
        sub_14000cec0(rcx, 0);
    }
    eax = 0;
    return;
}
