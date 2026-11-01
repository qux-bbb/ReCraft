// L1 structure recovery (13 blocks)
void sub_1400258d4_l1(void) {
    var_20 = 0;
    hstring = 0;
    rsi = rdx;
    var_30 = rcx;
    WindowsCreateStringReference("Windows.Storage.StorageFile", 0xADDR, &hstringHeader, &hstring, 0);
    if (eax < 0 /* signed */) {
        ecx = eax;
        sub_14000b320(eax);
    }
    RoGetActivationFactory(hstring, off_14002afa8, &var_20);
    ebx = eax;
    if (eax >= 0 /* signed */) {
        rbx = var_20;
        var_28 = 0;
        rax = *(u64*)(rbx);
        rdi = *(u64*)((rax + 0xADDR));
        sub_140009b18(&hstringHeader, &var_30);
        rdx = *(u64*)((rax + 0xADDR));
        rax = rdi;
        sub_140028010(rbx, *(u64*)((rax + 0xADDR)), &var_28);
        ebx = eax;
        if (eax >= 0 /* signed */) {
            rdi = var_28;
            sub_140025144(rdi, rdx, r8);
            ebx = eax;
            if (eax >= 0 /* signed */) {
                rax = *(u64*)(rdi);
                rdx = rsi;
                rcx = rdi;
                rax = *(u64*)((rax + 0xADDR));
                sub_140028010(rcx, rdx, r8);
                ebx = eax;
            }
        } else {
            r9d = eax;
            sub_140007cf0(var_78, 0xADDR, "shell\\osshell\\accesory\\common\\edpapphelper\\edpapphelper.cpp");
        }
        rcx = var_28;
        if (rcx != 0) {
            var_28 = 0;
            rax = *(u64*)(rcx);
            rax = *(u64*)((rax + 0xADDR));
            sub_140028010(rcx, rdx, r8);
        }
    } else {
        r9d = eax;
        sub_140007cf0(var_78, 0xADDR, "shell\\osshell\\accesory\\common\\edpapphelper\\edpapphelper.cpp");
    }
    rcx = var_20;
    if (rcx != 0) {
        var_20 = 0;
        rax = *(u64*)(rcx);
        rax = *(u64*)((rax + 0xADDR));
        sub_140028010(rcx, rdx, r8);
    }
    eax = ebx;
    return;
}
