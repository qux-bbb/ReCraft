// L1 structure recovery (4 blocks)
void sub_140001f0c_l1(void) {
    var_68 = 0;
    rbx = rcx;
    var_60 = 0;
    var_70 = 0;
    RtlCaptureContext(rcx);
    rsi = *(u64*)((rbx + 0xADDR));
    edi = 0;
    do {
        RtlLookupFunctionEntry(rsi, &var_60, 0);
        if (rax == 0) break;
        var_38 = 0;
        var_30 = &var_68;
        rcx = &var_70;
        var_28 = rcx;
        var_20 = rbx;
        RtlVirtualUnwind(0, var_60, rsi, rax, rbx, &var_70, &var_68, 0);
        edi += 1;
    } while (edi < 2 /* signed */);
    return;
}
