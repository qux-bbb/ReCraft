// L1 structure recovery (22 blocks)
void puts_l1(void) {
    var_60 = 0;
    var_70 = sil;
    ebx = (rsi + 1);
    var_88 = sil;
    var_90 = sil;
    var_98 = sil;
    if (dword_ADDR == 0) {
        xmm0 = xmmword_ADDR;
        var_88 = bl;
        var_78 = xmm0;
    }
    var_d8 = rcx;
    if (rcx != 0) {
        __acrt_iob_func(ebx);
        var_e0 = rax;
        ecx = (*(u32*)((rax + 0xADDR)) >> 0xADDR);
        if ((bl & cl) != 0) {
            rax = var_d8;
            rdi = -1;
            do {
                rdi += 1;
            } while (*(u8*)((rax + rdi)) != sil);
            rax = &var_e0;
            var_e8 = rdi;
            var_40 = rax;
            r9 = &var_30;
            var_48 = &var_60;
            var_50 = &var_d8;
            var_58 = &var_e8;
            rax = var_e0;
            var_30 = rax;
            var_38 = rax;
            sub_140004bdc(&var_d0, &var_38, &var_40);
            edi = eax;
        } else {
            sub_140009a28(rax);
            r8 = (int64)(eax);
            r9 = off_1400231d0;
            rdx = r8;
            ecx = (r8 + 2);
            if (ecx <= ebx) {
                r10 = r9;
            } else {
                rcx = r8;
                rax = (r8 >> 6);
                ecx &= 0xADDR;
                rax = *(u64*)((off_140023f50 + (rax * 8)));
                rcx += (rcx * 8);
                r10 = (rax + (rcx * 8));
            }
            if (*(u8*)((r10 + 0xADDR)) != sil) {
                rax = &var_60;
                var_90 = bl;
                var_28 = rax;
                r9d = 0;
                r8d = 0;
                var_20 = 0;
                edx = 0;
                var_8c = 0xADDR;
                ecx = 0;
                sub_140008bd0(0, 0, 0, 0);
                rdi = -1;
            } else {
                eax = (r8 + 2);
                if (eax > ebx) {
                    rax = (rdx >> 6);
                    edx &= 0xADDR;
                    rax = *(u64*)((off_140023f50 + (rax * 8)));
                    rcx = (rdx + (rdx * 8));
                    r9 = (rax + (rcx * 8));
                }
            }
        }
    }
    if (var_88 == 2) {
        rcx = var_60;
        *(u32*)((rcx + 0xADDR)) &= -0xADDR;
    }
    if (var_90 != sil) {
        ebx = var_8c;
        sub_140004828(&var_60);
        *(u32*)((rax + 0xADDR)) = ebx;
    }
    if (var_98 != sil) {
        ebx = var_94;
        sub_140004828(&var_60);
        *(u32*)((rax + 0xADDR)) = ebx;
    }
    eax = edi;
    return;
}
