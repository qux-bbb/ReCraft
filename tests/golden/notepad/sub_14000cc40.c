// L1 structure recovery (16 blocks)
void sub_14000cc40_l1(void) {
    var_30 = 0;
    rbx = rcx;
    var_34 = 0;
    edx -= 0xADDR;
    if (edx == 0) {
        SendMessageW(qword_ADDR, 0xADDR, &var_30, &var_34);
        SendMessageW(qword_ADDR, 0xADDR, var_30, 0);
        sub_1400083f8(&var_40, 0xADDR, "%d", (rax + 1));
        SetDlgItemTextW(rbx, 0xADDR, &var_40);
        SetFocus(rbx);
        eax = 1;
    } else if (edx != 1) {
        eax = 0;
    } else {
        ecx = (uint32)(r8w);
        ecx -= edx;
        if (ecx == 0) {
            GetDlgItemTextW(rbx, 0xADDR, &var_40, 0xADDR);
            var_20 = 0xADDR;
            FoldStringW(0xADDR, &var_40, (-1 | -1), &var_40, 0xADDR);
            _o__wtol(&var_40);
            rcx = qword_ADDR;
            dword_ADDR = eax;
            SendMessageW(qword_ADDR, 0xADDR, (rax - 1), 0);
            if (dword_ADDR <= 0) {
                MessageBoxW(rbx, qword_ADDR, qword_ADDR, 0);
                SendMessageW(qword_ADDR, 0xADDR, 0, 0);
                sub_1400083f8(&var_40, 0xADDR, "%d", eax);
                SetDlgItemTextW(rbx, 0xADDR, &var_40);
                SetFocus(rbx);
            } else if (eax == -1) {
                MessageBoxW(rbx, qword_ADDR, qword_ADDR, 0);
                SendMessageW(qword_ADDR, 0xADDR, 0, 0);
                sub_1400083f8(&var_40, 0xADDR, "%d", eax);
                SetDlgItemTextW(rbx, 0xADDR, &var_40);
                SetFocus(rbx);
                eax = 0;
                return;
            }
            edx = 0;
            EndDialog(rbx);
            eax = 1;
            return;
        }
        ecx -= edx;
        if (ecx == 0) {
            edx = 1;
        } else if (ecx != 0xADDR) {
            eax = 0;
            return;
        }
        eax = 1;
        return;
    }
    return;
}
