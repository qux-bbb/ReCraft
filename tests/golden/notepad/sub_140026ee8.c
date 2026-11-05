// L1 structure recovery (21 blocks)
void sub_140026ee8_l1(void) {
    rdx = off_14002a790;
    sub_140001c90(0xADDR);
    rbx = rax;
    if (rax == 0) {
        ebx = 0;
    } else {
        rcx = qword_ADDR;
        *(u64*)(0) = "@)";
        *(u64*)(0) = 0xADDR;
        *(u32*)(0xADDR) = 1;
        if (rcx != 0) {
            rax = *(u64*)(rcx);
            rax = *(u64*)((rax + 8));
            sub_140028010(rcx, rdx, r8);
        }
        *(u64*)((rbx + 0xADDR)) = 0xADDR;
        *(u64*)(rbx) = 0xADDR;
    }
    var_20 = 0;
    hstring = 0;
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
        sub_140028010(rcx, rdx, r8);
    }
    RoGetActivationFactory(rdi, 0xADDR, &var_20);
    edi = eax;
    if (eax >= 0 /* signed */) {
        if (rbx == 0) {
            edi = 0xADDR;
        } else {
            rcx = var_20;
            rax = *(u64*)(rcx);
            rax = *(u64*)((rax + 0xADDR));
            sub_140028010(var_20, rbx, 0xADDR);
            edi = eax;
        }
        rcx = var_20;
        if (rcx != 0) {
            var_20 = 0;
            rax = *(u64*)(rcx);
            rax = *(u64*)((rax + 0xADDR));
            sub_140028010(rcx, rdx, r8);
        }
    } else {
        r9d = eax;
        sub_140007cf0(var_58, 0xADDR, "shell\\osshell\\accesory\\common\\edpapphelper\\edpapphelper.cpp");
        rcx = var_20;
        if (rcx != 0) {
            var_20 = 0;
            rdx = *(u64*)(rcx);
            rax = *(u64*)((rdx + 0xADDR));
            sub_140028010(rcx, rdx, r8);
            eax = edi;
            return;
        }
    }
    if (rbx != 0) {
        rax = *(u64*)(rbx);
        rcx = rbx;
        rax = *(u64*)((rax + 0xADDR));
        sub_140028010(rcx, rdx, r8);
    }
    eax = edi;
    return;
}
