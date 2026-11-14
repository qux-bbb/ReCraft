// L1 structure recovery (18 blocks)
void sub_140025a60_l1(void) {
    var_68 = rdx;
    rax = *(u64*)(r8);
    r14 = rcx;
    r15d = 0;
    var_60 = 0;
    rax = *(u64*)((rax + 0xADDR));
    sub_140028010(r8, &var_60);
    ebx = eax;
    if (eax >= 0 /* signed */) {
        rcx = var_60;
        var_68 = 0;
        rax = *(u64*)(rcx);
        rax = *(u64*)((rax + 0xADDR));
        sub_140028010(var_60, &var_68);
        ebx = eax;
        if (eax >= 0 /* signed */) {
            esi = 0;
            if (var_68 <= r15d) {
                rcx = var_60;
                if (rcx != 0) {
                    var_60 = r15;
                    rax = *(u64*)(rcx);
                    rax = *(u64*)((rax + 0xADDR));
                    sub_140028010(rcx, rdx, r8);
                }
                ebx = r15d;
            } else {
                while (1) {
                    rbx = var_60;
                    rax = *(u64*)(rbx);
                    rdi = *(u64*)((rax + 0xADDR));
                    WindowsDeleteString(0);
                    hstring = r15;
                    rax = rdi;
                    sub_140028010(rbx, esi, &hstring);
                    ebx = eax;
                    if (eax < 0 /* signed */) break;
                    WindowsGetStringRawBuffer(*(u64*)((r14 + 0xADDR)), 0);
                    rbx = rax;
                    WindowsGetStringRawBuffer(hstring, 0);
                    r9d = -1;
                    var_10 = 1;
                    edx |= -1;
                    CompareStringOrdinal(rax, (edx | r9d), rbx, (r9d | -1), 1);
                    if (eax != 2) {
                        WindowsDeleteString(hstring);
                        esi += 2;
                        hstring = r15;
                        if (esi < var_68) continue;
                    }
                }
                r9d = eax;
                sub_140007cf0(var_58, 0xADDR, "shell\\osshell\\accesory\\common\\edpapphelper\\edpapphelper.cpp");
                WindowsDeleteString(hstring);
                hstring = r15;
                rcx = var_60;
                if (rcx != 0) {
                    var_60 = r15;
                    rax = *(u64*)(rcx);
                    rax = *(u64*)((rax + 0xADDR));
                    sub_140028010(rcx, rdx, r8);
                }
                ecx = ebx;
                sub_140025680(ebx);
            }
        } else {
            edx = 0xADDR;
            r9d = eax;
            sub_140007cf0(var_58, rdx, "shell\\osshell\\accesory\\common\\edpapphelper\\edpapphelper.cpp");
            rcx = var_60;
            if (rcx != 0) {
                var_60 = r15;
                rax = *(u64*)(rcx);
                rax = *(u64*)((rax + 0xADDR));
                sub_140028010(rcx, rdx, r8);
            }
            ecx = ebx;
            sub_140025680(ebx);
            eax = ebx;
            return;
        }
    } else {
        edx = 0xADDR;
    }
    eax = ebx;
    return;
}
