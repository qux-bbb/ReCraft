// L1 structure recovery (25 blocks)
void sub_14000f9f0_l1(void) {
    sub_14000fb98(1);
    SendMessageW(qword_ADDR, 0xADDR, 0, 0);
    edx = 0;
    rdi = rax;
    sub_1400029dc(&var_20, 0, 0xADDR);
    edx = (edi - 1);
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
    sub_1400083f8(&var_20, 0xADDR, r8, r9);
    if (eax >= 0 /* signed */) {
        rcx = qword_ADDR;
        if (rcx != 0) {
            SendMessageW(qword_ADDR, 0xADDR, (rbx - 0xADDR), &var_20);
        }
    }
    edx = 0;
    dword_ADDR = edi;
    sub_1400029dc(&var_20, 0, 0xADDR);
    ecx = dword_ADDR;
    if (ecx != 6) {
        switch (ecx) {
            case 0xADDR: {
                ecx -= 0xADDR;
                r9 = qword_ADDR;
                break;
            }
            case 0xADDR: {
                ecx -= 0xADDR;
                r9 = qword_ADDR;
                break;
            }
            case 0xADDR: {
                ecx -= 0xADDR;
                r9 = qword_ADDR;
                break;
            }
            case 0xADDR: {
                ecx -= 0xADDR;
                r9 = qword_ADDR;
                break;
            }
            case 0xADDR: {
                ecx -= 0xADDR;
                r9 = qword_ADDR;
                break;
            }
            default: {
                ecx -= 0xADDR;
                break;
            }
        }
    } else {
        r9 = "GB18030";
    }
    sub_1400083f8(&var_20, rbx, qword_ADDR, r9);
    if (eax >= 0 /* signed */) {
        rcx = qword_ADDR;
        if (rcx != 0) {
            SendMessageW(qword_ADDR, 0xADDR, 4, &var_20);
        }
    }
    eax = dword_ADDR;
    dword_ADDR = eax;
    sub_14000fd5c();
    r11 = &var_b0;
    return;
}
