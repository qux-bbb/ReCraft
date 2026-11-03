// L1 structure recovery (10 blocks)
void sub_14001008c_l1(void) {
    rsi = rdx;
    edx = -1;
    rdi = rcx;
    DragQueryFileW(rcx, (edx | -1), 0, 0);
    if (eax != 0) {
        DragQueryFileW(rdi, 0, 0, 0);
        if (eax != 0) {
            ebx = (rax + 1);
            edx = 0;
            sub_140009e44(&var_60, 0, (int64)(ebx));
            r9d = ebx;
            rbx = var_60;
            DragQueryFileW(rdi, 0, rbx, ebx);
            SetActiveWindow(rsi);
            rsi = qword_ADDR;
            sub_14000c520();
            if (eax != 0) {
                var_30 = 0;
                var_28 = 0xADDR;
                var_20 = 3;
                CreateFileW(rbx, 0xADDR, 3, 0, 3, 0xADDR, 0);
                qword_ADDR = rax;
                dword_ADDR = 1;
                if (rax == -1) {
                    sub_140010b30(rbx);
                    qword_ADDR = rsi;
                } else {
                    r8d = 0;
                    edx = 0;
                    sub_140010f48(&var_60, 0, 0);
                    rbx = var_60;
                }
                dword_ADDR = 0;
            }
            if (rbx != 0) {
                CoTaskMemFree(rbx);
            }
        }
    }
    rcx = rdi;
    DragFinish(rdi);
    return;
}
