// L1 structure recovery (7 blocks)
void sub_1400221ac_l1(void) {
    r15 = rcx;
    var_78 = 0;
    CoCreateGuid(&var_78);
    if (eax >= 0 /* signed */) {
        edx = 0;
        sub_140009e44(&var_70, 0, (rdx + 0xADDR));
        ecx = (uint32)(var_86);
        r8d = (uint32)(var_85);
        r9d = (uint32)(var_84);
        ebx = (uint32)(var_81);
        eax = (uint32)(var_87);
        r10d = (uint32)(var_83);
        r11d = (uint32)(var_82);
        edi = (uint32)(var_80);
        esi = (uint32)(var_7e);
        r14d = (uint32)(var_7c);
        var_68 = eax;
        var_60 = ecx;
        var_58 = r8d;
        var_50 = r9d;
        var_48 = r10d;
        var_40 = r11d;
        var_38 = ebx;
        rbx = var_70;
        var_30 = edi;
        var_28 = esi;
        var_20 = r14d;
        sub_1400083f8(rbx, 0xADDR, "%08lX-%04X-%04x-%02X%02X-%02X%02X%02X%02X%02X%02X", var_78, (uint32)(var_7c), (uint32)(var_7e), (uint32)(var_80), (uint32)(var_81), (uint32)(var_82), (uint32)(var_83), (uint32)(var_84), (uint32)(var_85), (uint32)(var_86));
        if (eax >= 0 /* signed */) {
            *(u64*)(r15) = rbx;
        } else {
            *(u64*)(r15) = 0;
            if (rbx != 0) {
                CoTaskMemFree(rbx);
            }
        }
    } else {
        *(u64*)(r15) = 0;
    }
    rax = r15;
    r11 = &var_90;
    return;
}
