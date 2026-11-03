// L1 structure recovery (17 blocks)
void sub_140026734_l1(void) {
    var_b0 = r8d;
    r14d = 0;
    rsi = rdx;
    *(u16*)(rdx) = r14w;
    rax = *(u64*)(rcx);
    var_a0 = 0;
    rax = *(u64*)((rax + 0xADDR));
    sub_140028010(rcx, &var_a0);
    ebx = eax;
    if (eax >= 0 /* signed */) {
        rbx = var_a0;
        if (rbx != 0) {
            rax = *(u64*)(rbx);
            rcx = rbx;
            rax = *(u64*)((rax + 8));
            sub_140028010(rcx, rdx, r8);
        }
        xmm0 = xmmword_ADDR;
        rax = &var_a8;
        var_a8 = r14;
        var_40 = rax;
        rax = &var_b0;
        var_b0 = r14d;
        var_48 = rax;
        eax = *(u32*)("\"");
        var_30 = eax;
        rax = *(u64*)(rbx);
        var_50 = 1;
        var_20 = xmm0;
        var_58 = r14w;
        rax = *(u64*)((rax + 0xADDR));
        sub_140028010(rbx, &var_20, &var_58);
        edi = eax;
        if (eax >= 0 /* signed */) {
            PropVariantToStringVectorAlloc(&var_58, &var_a8, &var_b0);
            edi = eax;
        }
        PropVariantClear(&var_58);
        if (edi < 0 /* signed */) {
            sub_1400253cc(&var_40);
            rax = *(u64*)(rbx);
            rcx = rbx;
            rax = *(u64*)((rax + 0xADDR));
            sub_140028010(rcx, rdx, r8);
            rcx = var_a0;
            if (rcx != 0) {
                var_a0 = r14;
                rax = *(u64*)(rcx);
                rax = *(u64*)((rax + 0xADDR));
                sub_140028010(rcx, rdx, r8);
            }
        } else if (var_b0 > r14d) {
            r8 = var_a8;
            r8 = *(u64*)(r8);
            sub_140008384(rsi, 0xADDR);
            edi = 0;
            sub_1400253cc(&var_40);
            rcx = *(u64*)(rbx);
            rax = *(u64*)((rcx + 0xADDR));
            rcx = rbx;
            sub_140028010(rbx);
            rcx = var_a0;
            if (rcx != 0) {
                var_a0 = r14;
                rdx = *(u64*)(rcx);
                rax = *(u64*)((rdx + 0xADDR));
                sub_140028010(rcx, *(u64*)(rcx));
            }
            eax = edi;
        }
    } else {
        r9d = eax;
        sub_140007cf0(var_98, 0xADDR, "shell\\osshell\\accesory\\common\\edpapphelper\\edpapphelper.cpp");
        rcx = var_a0;
        if (rcx != 0) {
            var_a0 = r14;
            rdx = *(u64*)(rcx);
            rax = *(u64*)((rdx + 0xADDR));
            sub_140028010(rcx, *(u64*)(rcx));
        }
        eax = ebx;
    }
    return;
}
