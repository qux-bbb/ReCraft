// L1 structure recovery (8 blocks)
void __scrt_fastfail_l1(void) {
    ebx = ecx;
    IsProcessorFeaturePresent(0xADDR);
    if (eax == 0) {
        sub_140001794(3);
        edx = 0;
        sub_140016120(&var_f0, 0, 0xADDR);
        RtlCaptureContext(&var_f0);
        rbx = var_1e8;
        RtlLookupFunctionEntry(rbx, &var_5d8, 0);
        if (rax != 0) {
            rcx = &var_5e0;
            var_38 = 0;
            var_30 = rcx;
            var_28 = &var_5e8;
            rcx = &var_f0;
            var_20 = rcx;
            RtlVirtualUnwind(0, var_5d8, rbx, rax, &var_f0, &var_5e8, &var_5e0, 0);
        }
        var_1e8 = var_5c8;
        edx = 0;
        rax = (&var_5c8 + 8);
        var_188 = rax;
        sub_140016120(&var_50, 0, 0xADDR);
        rax = var_5c8;
        var_60 = rax;
        var_50 = 0xADDR;
        var_54 = 1;
        IsDebuggerPresent();
        ebx = eax;
        var_40 = &var_50;
        rax = &var_f0;
        var_48 = rax;
        SetUnhandledExceptionFilter(0);
        UnhandledExceptionFilter(&var_40);
        if (eax == 0 && ebx != 1) {
            sub_140001794((rax + 3));
        }
        return;
    }
    ecx = ebx;
    __asm("int 0xADDR");
}
