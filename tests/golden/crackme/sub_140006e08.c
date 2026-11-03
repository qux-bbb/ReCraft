// L1 structure recovery (16 blocks)
void sub_140006e08_l1(void) {
    var_30 = 0;
    var_40 = 0;
    var_58 = 0;
    var_60 = 0;
    var_68 = 0;
    if (r9 == 0) {
        if (dword_ADDR == 0) {
            xmm0 = xmmword_ADDR;
            var_58 = 1;
            var_48 = xmm0;
        }
    } else {
        xmm0 = *(__m128*)(r9);
        var_58 = 1;
        var_48 = xmm0;
    }
    rax = var_f0;
    var_e8 = rax;
    var_e0 = r8;
    var_d0 = rdx;
    var_d8 = rcx;
    if (rdx != 0) {
        if (r8 == 0) {
            rax = &var_30;
            var_60 = 1;
            var_28 = rax;
            r9d = 0;
            var_20 = 0;
            r8d = 0;
            edx = 0;
            var_5c = 0xADDR;
            ecx = 0;
            sub_140008bd0(0, 0, 0, 0);
            edi = -1;
        } else {
            rax = &var_d0;
            var_70 = 0;
            var_80 = rax;
            r9 = &var_70;
            rax = &var_30;
            var_78 = rdx;
            var_88 = rax;
            var_90 = &var_d8;
            var_98 = &var_e0;
            rax = &var_e8;
            var_a0 = rax;
            sub_140004e6c(&var_f0, &var_78, &var_80);
            edi = eax;
        }
    }
    if (var_58 == 2) {
        rcx = var_30;
        *(u32*)((rcx + 0xADDR)) &= -0xADDR;
    }
    if (var_60 != 0) {
        ebx = var_5c;
        sub_140004828(&var_30);
        *(u32*)((rax + 0xADDR)) = ebx;
    }
    if (var_68 != 0) {
        ebx = var_64;
        sub_140004828(&var_30);
        *(u32*)((rax + 0xADDR)) = ebx;
    }
    eax = edi;
    return;
}
