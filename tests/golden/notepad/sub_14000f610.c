// L1 structure recovery (19 blocks)
void sub_14000f610_l1(void) {
    r14d = 0;
    var_30 = rcx;
    rsi = rcx;
    var_20 = r14b;
    var_28 = 0;
    hstring = 0;
    WindowsCreateStringReference("Windows.Security.EnterpriseData.ProtectionPolicyManager", (r14 + 0xADDR), &hstringHeader, &hstring, 0, 0, rcx);
    if (eax < 0 /* signed */) {
        ecx = eax;
        sub_14000b320(eax);
    }
    RoGetActivationFactory(hstring, off_14002e110, &var_28);
    eax >>= 0xADDR;
    al ^= 1;
    hstring = 0;
    if (al != 0) {
        rbx = var_28;
        rax = *(u64*)(rbx);
        rdi = *(u64*)((rax + 0xADDR));
        sub_140009b18(&var_58, &var_30);
        rdx = *(u64*)((rax + 0xADDR));
        rax = rdi;
        sub_140028010(rbx, *(u64*)((rax + 0xADDR)), &var_20);
        var_70 = r14;
    }
    rcx = var_28;
    ebx = (uint32)(var_20);
    if (rcx != 0) {
        var_28 = r14;
        rax = *(u64*)(rcx);
        rax = *(u64*)((rax + 0xADDR));
        sub_140028010(rcx, rdx, r8);
    }
    if (ebx == 0) {
        eax = 0;
    } else {
        rcx = off_140036200;
        r8 = (rsi - off_140036200);
        edx = 0xADDR;
        do {
            rax = (rdx + 0xADDR);
            if (rax == 0) break;
            eax = (uint32)(*(u16*)((r8 + rcx)));
            if (ax == 0) break;
            *(u16*)(rcx) = ax;
            rcx += 2;
            rdx -= 1;
        } while (rdx != 0);
        rax = (rcx - 2);
        rax = ((rdx != 0) ? rcx : rax);
        *(u16*)(rax) = r14w;
        if (rdx == 0) {
            eax = 0;
            r11 = &var_80;
            return;
        }
        rcx = qword_ADDR;
        dword_ADDR = 2;
        if (rcx != 0) {
            sub_14002708c(rcx);
            qword_ADDR = r14;
        }
        sub_140026c44(rsi);
        if (eax < 0 /* signed */) {
            qword_ADDR = r14;
        }
        eax = 1;
    }
    r11 = &var_80;
    return;
}
