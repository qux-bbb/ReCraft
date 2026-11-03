// L1 structure recovery (15 blocks)
void sub_140009798_l1(void) {
    rdi = var_a0;
    ebx = 0;
    rax = var_a8;
    r15d = edx;
    rsi = rcx;
    if (rdi != 0) {
        *(u32*)(rdi) = ebx;
    }
    *(u32*)(rax) = 1;
    r14d = 0;
    var_30 = 0;
    r14b = (uint8)((r8d == 0));
    ebp = ebx;
    var_38 = 0;
    var_40 = 0;
    rax = qword_ADDR;
    if (rax != 0) {
        sub_140028010(r15d, r14d, &var_30, &var_38);
        edx = eax;
        ecx = eax;
        if (eax != 0) {
            if (ecx == 0xADDR) {
                eax = ((var_3c >> 7) & 1);
                *(u32*)((rsi + 0xADDR)) = eax;
                ebp = 1;
            }
        } else {
            ecx = var_3c;
            eax = ((ecx >> 4) & 3);
            *(u32*)(rsi) = eax;
            eax = (ecx >> 8);
            al &= 0xADDR;
            *(u8*)((rsi + 4)) = al;
            *(u32*)((rsi + 0xADDR)) = var_40;
            eax = ((ecx >> 0xADDR) & 3);
            *(u32*)((rsi + 8)) = eax;
            eax = ((ecx >> 6) & 1);
            ecx >>= 7;
            ecx &= 1;
            *(u32*)((rsi + 0xADDR)) = eax;
            *(u32*)((rsi + 0xADDR)) = ecx;
            ebp = 1;
        }
    } else {
        rax = qword_ADDR;
        if (rax == 0) {
            GetModuleHandleW("ntdll.dll");
            qword_ADDR = rax;
        }
        GetProcAddress(rax, "RtlQueryFeatureConfiguration");
        qword_ADDR = rax;
        if (rax == 0) {
            edx = 0xADDR;
        }
    }
    if (rdi != 0) {
        bl = (uint8)((edx != 0xADDR));
        *(u32*)(rdi) = ebx;
    }
    eax = ebp;
    return;
}
