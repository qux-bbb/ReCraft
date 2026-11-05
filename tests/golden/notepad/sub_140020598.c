// L1 structure recovery (11 blocks)
void sub_140020598_l1(void) {
    rbx = qword_ADDR;
    if (rbx == 0) {
        dword_ADDR |= 0xADDR;
        sub_14000f5b8(off_140035550);
        dword_ADDR &= 0xADDR;
        rbx = qword_ADDR;
    }
    sub_1400029dc(&var_20, 0, 0xADDR);
    rax = qword_ADDR;
    var_a0 = -1;
    rcx = var_30;
    var_28 = rax;
    rax = qword_ADDR;
    var_20 = 0xADDR;
    var_48 = 0xADDR;
    rcx = ((rax != 0) ? rax : rcx);
    rax = var_38;
    var_30 = rcx;
    rax = ((rbx != 0) ? rbx : rax);
    ebx = 0;
    var_38 = rax;
    PrintDlgExW(&var_20);
    if (eax == 0 && var_a4 == 1) {
        rbx = var_40;
        if (qword_ADDR == 0) {
            dword_ADDR |= 0xADDR;
            sub_14000f5b8(off_140035550);
            dword_ADDR &= 0xADDR;
        }
        qword_ADDR = var_30;
        qword_ADDR = var_38;
    } else if (var_a4 == 2) {
        if (qword_ADDR == 0) {
            dword_ADDR |= 0xADDR;
            sub_14000f5b8(off_140035550);
            dword_ADDR &= 0xADDR;
        }
        qword_ADDR = var_30;
        qword_ADDR = var_38;
        rax = rbx;
        return;
    }
    rax = rbx;
    return;
}
