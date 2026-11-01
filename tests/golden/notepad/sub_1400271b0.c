// L1 structure recovery (12 blocks)
void sub_1400271b0_l1(void) {
    var_20 = 0;
    hstring = 0;
    rbx = rcx;
    WindowsCreateStringReference("Windows.Security.EnterpriseData.ProtectionPolicyManager", 0xADDR, &hstringHeader, &hstring, 0);
    if (eax < 0 /* signed */) {
        ecx = eax;
        sub_14000b320(eax);
    }
    rcx = var_20;
    rdi = hstring;
    if (rcx != 0) {
        rax = *(u64*)(rcx);
        rax = *(u64*)((rax + 0xADDR));
        sub_140028010(rcx, rdx);
    }
    RoGetActivationFactory(rdi, off_14002e1b0, &var_20);
    edi = eax;
    if (eax >= 0 /* signed */) {
        rcx = var_20;
        rdx = rbx;
        rax = *(u64*)(rcx);
        rax = *(u64*)((rax + 0xADDR));
        sub_140028010(var_20);
        rcx = var_20;
        ebx = eax;
        if (rcx != 0) {
            var_20 = 0;
            rdx = *(u64*)(rcx);
            rax = *(u64*)((rdx + 0xADDR));
            sub_140028010(var_n30, *(u64*)(rcx));
        }
        eax = ebx;
    } else {
        r9d = eax;
        sub_140007cf0(var_58, 0xADDR, "shell\\osshell\\accesory\\common\\edpapphelper\\edpapphelper.cpp");
        rcx = var_20;
        if (rcx != 0) {
            var_20 = 0;
            rdx = *(u64*)(rcx);
            rax = *(u64*)((rdx + 0xADDR));
            sub_140028010(var_n30, *(u64*)(rcx));
        }
        eax = edi;
    }
    return;
}
