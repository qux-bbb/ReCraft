// L1 structure recovery (22 blocks)
void sub_140001840_l1(void) {
    sub_140001a54(1);
    if (al == 0) {
        sub_140002100(7);
        sub_140002100(7);
        ecx = ebx;
        sub_140002988(ebx);
        ecx = ebx;
        sub_140002904(ebx);
    }
    dil = 0;
    var_20 = dil;
    sub_140001a14();
    bl = al;
    ecx = dword_ADDR;
    if (ecx == 1) {
        sub_140002100(7);
        ecx = ebx;
        sub_140002988(ebx);
        ecx = ebx;
        sub_140002904(ebx);
        return;
    }
    if (ecx != 0) {
        dil = 1;
        var_20 = dil;
        sub_140001bc8(bl);
        sub_1400020d0();
        rbx = rax;
        if (*(u64*)(rax) != 0) {
            sub_140001b28(rax);
            if (al != 0) {
                r8d = 0;
                ecx = 0;
                rax = *(u64*)(rbx);
                sub_140028010(0, (r8 + 2));
            }
        }
        sub_1400020e0();
        rbx = rax;
        if (*(u64*)(rax) != 0) {
            sub_140001b28(rax);
            if (al != 0) {
                sub_140002880(*(u64*)(rbx));
            }
        }
        sub_140002268();
        ebx = (uint32)(ax);
        sub_140002910();
        r9d = ebx;
        r8 = rax;
        edx = 0;
        sub_1400101d8(0xADDR, 0);
        ebx = eax;
        sub_1400022b4();
        if (al == 0) {
            ecx = ebx;
            sub_140002988(ebx);
            ecx = ebx;
            sub_140002904(ebx);
            return;
        }
        if (dil == 0) {
            sub_1400028d4();
        }
        edx = 0;
        sub_140001bf4(1);
        eax = ebx;
        return;
    }
    dword_ADDR = 1;
    sub_140002868(off_14002a508, off_14002a530);
    if (eax == 0) {
        sub_14000285c(off_14002a470, off_14002a500);
        dword_ADDR = 2;
        sub_140001bc8(bl);
        sub_1400020d0();
        rbx = rax;
        if (*(u64*)(rax) != 0) {
            sub_140001b28(rax);
            if (al != 0) {
                r8d = 0;
                ecx = 0;
                rax = *(u64*)(rbx);
                sub_140028010(0, (r8 + 2));
            }
        }
        sub_1400020e0();
        rbx = rax;
        if (*(u64*)(rax) != 0) {
            sub_140001b28(rax);
            if (al != 0) {
                sub_140002880(*(u64*)(rbx));
            }
        }
        sub_140002268();
        ebx = (uint32)(ax);
        sub_140002910();
        r9d = ebx;
        r8 = rax;
        edx = 0;
        sub_1400101d8(0xADDR, 0);
        ebx = eax;
        sub_1400022b4();
        if (al == 0) {
            ecx = ebx;
            sub_140002988(ebx);
            ecx = ebx;
            sub_140002904(ebx);
            return;
        }
        if (dil == 0) {
            sub_1400028d4();
        }
        edx = 0;
        sub_140001bf4(1);
        eax = ebx;
        return;
    }
    eax = 0xADDR;
    return;
}
