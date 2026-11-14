// L1 structure recovery (16 blocks)
void sub_14000491c_l1(void) {
    var_40 = 0;
    rbx = rcx;
    var_30 = 0;
    var_18 = 0;
    var_10 = 0;
    var_8 = 0;
    if (dword_ADDR == 0) {
        xmm0 = xmmword_ADDR;
        var_18 = 1;
        var_28 = xmm0;
    }
    if (rbx != 0) {
        sub_140004890(rcx, &var_40);
        if (eax == 0) {
            eax = (*(u32*)((rbx + 0xADDR)) >> 0xADDR);
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
        sub_1400047c8(0);
        edi = eax;
    }
    if (var_18 == 2) {
        rax = var_40;
        *(u32*)((rax + 0xADDR)) &= -0xADDR;
    }
    if (var_10 != 0) {
        ebx = var_14;
        sub_140004828(&var_40);
        *(u32*)((rax + 0xADDR)) = ebx;
    }
    if (var_8 != 0) {
        ebx = var_c;
        sub_140004828(&var_40);
        *(u32*)((rax + 0xADDR)) = ebx;
    }
    eax = edi;
    return;
}
