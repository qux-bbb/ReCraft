// L1 structure recovery (16 blocks)
void sub_14000491c_l1(void) {
    var_20 = 0;
    rbx = rcx;
    var_30 = 0;
    var_48 = 0;
    var_50 = 0;
    var_58 = 0;
    if (dword_ADDR == 0) {
        xmm0 = xmmword_ADDR;
        var_48 = 1;
        var_38 = xmm0;
    }
    if (rbx != 0) {
        sub_140004890(rcx, &var_20);
        if (eax == 0) {
            eax = *(u32*)((rbx + 0xADDR));
            eax >>= 0xADDR;
            if ((al & 1) == 0) {
                edi = 0;
            } else {
                sub_140009a28(rbx);
                sub_140009994(eax);
                if (eax != 0) {
                    edi = -1;
                }
            }
        }
    } else {
        ecx = 0;
        sub_1400047c8(0);
        edi = eax;
    }
    if (var_48 == 2) {
        rax = var_20;
        *(u32*)((rax + 0xADDR)) = (*(u32*)((rax + 0xADDR)) & -0xADDR);
    }
    if (var_50 != 0) {
        ebx = var_4c;
        sub_140004828(&var_20);
        *(u32*)((rax + 0xADDR)) = ebx;
    }
    if (var_58 != 0) {
        ebx = var_54;
        sub_140004828(&var_20);
        *(u32*)((rax + 0xADDR)) = ebx;
    }
    eax = edi;
    return;
}
