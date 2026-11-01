// L1 structure recovery (8 blocks)
void sub_1400268d4_l1(void) {
    var_20 = 0;
    hstring = 0;
    WindowsCreateStringReference("Windows.Security.EnterpriseData.ProtectionPolicyManager", 0xADDR, &hstringHeader, &hstring, 0);
    if (eax < 0 /* signed */) {
        ecx = eax;
        sub_14000b320(eax);
    }
    RoGetActivationFactory(hstring, off_14002e110, &var_20);
    ebx = eax;
    if (eax >= 0 /* signed */) {
        rcx = var_20;
        rax = *(u64*)(rcx);
        rax = *(u64*)((rax + 0xADDR));
        sub_140028010(var_20);
        ebx = eax;
    } else {
        r9d = eax;
        sub_140007cf0(var_58, 0xADDR, "shell\\osshell\\accesory\\common\\edpapphelper\\edpapphelper.cpp");
    }
    rcx = var_20;
    if (rcx != 0) {
        var_20 = 0;
        rax = *(u64*)((*(u64*)(rcx) + 0xADDR));
        sub_140028010(var_20, *(u64*)(rcx));
    }
    eax = ebx;
    return;
}
