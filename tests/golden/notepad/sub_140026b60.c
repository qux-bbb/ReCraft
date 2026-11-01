// L1 structure recovery (9 blocks)
void sub_140026b60_l1(void) {
    var_28 = 0;
    hstring = 0;
    var_20 = 0;
    WindowsCreateStringReference("Windows.Security.EnterpriseData.ProtectionPolicyManager", 0xADDR, &hstringHeader, &hstring, 0, 0);
    if (eax < 0 /* signed */) {
        ecx = eax;
        sub_14000b320(eax);
    }
    rcx = var_28;
    rbx = hstring;
    if (rcx != 0) {
        rax = *(u64*)(rcx);
        rax = *(u64*)((rax + 0xADDR));
        sub_140028010(rcx, rdx);
    }
    RoGetActivationFactory(rbx, off_14002e1b0, &var_28);
    if (eax >= 0 /* signed */) {
        rcx = var_28;
        rax = *(u64*)(rcx);
        rax = *(u64*)((rax + 0xADDR));
        sub_140028010(var_28, &var_20);
    }
    rcx = var_28;
    ebx = (uint32)(var_20);
    if (rcx != 0) {
        var_28 = 0;
        rax = *(u64*)((*(u64*)(rcx) + 0xADDR));
        sub_140028010(var_n38, *(u64*)(rcx));
    }
    eax = ebx;
    return;
}
