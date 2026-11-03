// L1 structure recovery (8 blocks)
void sub_14000cec0_l1(void) {
    ebp = 0xADDR;
    rdi = r8;
    rbx = rcx;
    esi = (rbp - 0xADDR);
    if (dl != 0) {
        rax = qword_ADDR;
        *(u64*)((r11 - 0xADDR)) = rax;
        SendDlgItemMessageW(rcx, esi, ebp, 0);
    }
    rax = qword_ADDR;
    var_20 = rax;
    SendDlgItemMessageW(rbx, esi, ebp, 0, qword_ADDR);
    rax = qword_ADDR;
    var_20 = rax;
    SendDlgItemMessageW(rbx, esi, ebp, 0, qword_ADDR);
    rax = qword_ADDR;
    var_20 = rax;
    SendDlgItemMessageW(rbx, esi, ebp, 0, qword_ADDR);
    rax = qword_ADDR;
    var_20 = rax;
    SendDlgItemMessageW(rbx, esi, ebp, 0, qword_ADDR);
    rax = qword_ADDR;
    var_20 = rax;
    SendDlgItemMessageW(rbx, esi, ebp, 0, qword_ADDR);
    GetACP();
    if (eax == 0xADDR) {
        rax = "GB18030";
        var_20 = rax;
        SendDlgItemMessageW(rbx, esi, ebp, 0, "GB18030");
    }
    var_20 = rdi;
    ebp = 0xADDR;
    rdi = -1;
    SendDlgItemMessageW(rbx, esi, ebp, rdi, rdi);
    if (eax != edi) {
        var_20 = 0;
        SendDlgItemMessageW(rbx, esi, 0xADDR, (int64)(eax), 0);
    } else {
        rax = qword_ADDR;
        var_20 = rax;
        SendDlgItemMessageW(rbx, esi, ebp, rdi, qword_ADDR);
        if (eax != edi) {
            var_20 = 0;
            SendDlgItemMessageW(rbx, esi, 0xADDR, (int64)(eax), 0);
            return;
        }
    }
    return;
}
