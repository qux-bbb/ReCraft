// L1 structure recovery (11 blocks)
void sub_140021fc4_l1(void) {
    ebp = 0;
    *(u64*)(rcx) = off_140029258;
    *(u64*)((rcx + 8)) = rbp;
    rdi = rcx;
    rax = *(u64*)(rdx);
    *(u64*)((rcx + 0xADDR)) = rax;
    *(u64*)(rdx) = rbp;
    *(u8*)((rcx + 0xADDR)) = bpl;
    var_48 = rbp;
    SHGetKnownFolderPath(off_14002e2d0, 0, 0, &var_48);
    if (eax < 0 /* signed */) {
        rcx = var_48;
        if (rcx != 0) {
            CoTaskMemFree(var_48);
        }
    } else {
        rdx = var_48;
        if (rdx != 0) {
            rbx = -1;
            do {
                rbx += 1;
            } while (*(u16*)((rdx + (rbx * 2))) != bp);
            sub_140009e44(&var_40, rdx, (rbx + 9));
            r11 = var_40;
            sub_140008384((r11 + (rbx * 2)), 9, "\\Notepad");
            if (eax >= 0 /* signed */) {
                sub_14000a0c0((rdi + 8), &var_40);
                r11 = var_40;
            }
            if (r11 == 0) {
                rcx = var_48;
                if (rcx != 0) {
                    CoTaskMemFree(var_48);
                }
                rax = rdi;
                return;
            }
        }
    }
    rax = rdi;
    return;
}
