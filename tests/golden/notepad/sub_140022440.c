// L1 structure recovery (15 blocks)
void sub_140022440_l1(void) {
    rdi = rcx;
    rcx = *(u64*)((rcx + 8));
    CreateDirectoryW(*(u64*)((rcx + 8)), 0);
    ebp = 0;
    if (eax != 0) {
        rsi = *(u64*)((rdi + 8));
        rbx = -1;
        do {
            rbx += 1;
        } while (*(u16*)((rsi + (rbx * 2))) != bp);
        edx = 0;
        sub_140009e44(&var_68, 0, (rbx + 0xADDR));
        rdx = (rbx + 0xADDR);
        rbx = var_68;
        rax = "probe.autosave";
        var_20 = rax;
        sub_1400083f8(rbx, (rbx + 0xADDR), "%s\\%s", rsi, "probe.autosave");
        if (eax < 0 /* signed */) {
            if (rbx != 0) {
                CoTaskMemFree(rbx);
            }
            al = 0;
        } else {
            var_30 = rbp;
            var_28 = 0xADDR;
            var_20 = 2;
            CreateFileW(rbx, 0xADDR, 0, 0, 2, 0xADDR, rbp);
            if (rax != -1) {
                CloseHandle(rax);
                var_60 = rbp;
                GetDiskFreeSpaceExW(*(u64*)((rdi + 8)), &var_60, 0, 0);
                if (eax != 0) {
                    r9d = 0;
                    SendMessageW(qword_ADDR, (r9 + 0xADDR), 0, 0);
                    ecx = (rax + 1);
                    rcx += rcx;
                    if (rcx <= var_60) {
                        if (rbx != 0) {
                            CoTaskMemFree(rbx);
                        }
                        al = 1;
                    }
                }
            }
        }
    } else {
        GetLastError();
        if (eax != 0xADDR) {
            al = 0;
            return;
        }
    }
    return;
}
