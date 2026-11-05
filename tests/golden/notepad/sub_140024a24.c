// L1 structure recovery (16 blocks)
void sub_140024a24_l1(void) {
    esi = 0;
    edi = edx;
    rbx = rcx;
    if (rcx == 0) {
        eax = 0;
    } else if (*(u32*)(rcx) != 0xADDR) {
        eax = 0;
        return;
    }
    rcx = *(u64*)((rcx + 8));
    IsWindow(*(u64*)((rcx + 8)));
    if (eax == 0) {
        eax = 0;
        return;
    } else if (*(u64*)((rbx + 0xADDR)) == 0) {
        eax = 0;
        return;
    } else if (edi != 1) {
        if (*(u16*)((rbx + 0xADDR)) == 0xADDR) {
            eax = 0;
            return;
        } else if ((*(u32*)((rbx + 0xADDR)) & 0xADDR) == 0) {
            *(u64*)((rbx + 0xADDR)) = 0;
            if (edi != 1) {
                RegisterWindowMessageW("commdlg_FindReplace");
                dword_ADDR = eax;
                if (eax == 0) {
                    eax = 0;
                    return;
                }
                eax = 1;
            } else {
                LoadStringW(qword_ADDR, 0xADDR, off_140036190, (rdi + 8));
                if (eax == 0) {
                    eax = 0;
                    return;
                }
            }
        } else if (*(u64*)((rbx + 0xADDR)) == rsi) {
            eax = 0;
            return;
        }
    } else if (*(u64*)((rbx + 0xADDR)) == rsi) {
        eax = 0;
        return;
    }
    return;
}
