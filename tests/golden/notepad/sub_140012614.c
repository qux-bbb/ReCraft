// L1 structure recovery (21 blocks)
void sub_140012614_l1(void) {
    *(u64*)(rdx) = 0;
    rbx = rdx;
    rsi = qword_ADDR;
    eax = *(u32*)(rsi);
    *(u32*)(rdx) = eax;
    eax &= 6;
    if (al != 6) {
        sub_140004d94();
        var_48 = 0;
        ebp = eax;
        rax = qword_ADDR;
        if (rax == 0) {
            rax = qword_ADDR;
            if (rax != 0) {
                sub_140028010(0xADDR, 1, &var_48);
                edx = eax;
            } else {
                edx = 0;
            }
        }
        r8d = edx;
        eax = edx;
        r8d &= 0xADDR;
        edx &= 0xADDR;
        ecx = r8d;
        r12d = 0xADDR;
        ecx &= 3;
        eax &= 0xADDR;
        ecx <<= 2;
        ecx |= eax;
        ecx <<= 2;
        ecx |= edx;
        edi = (rcx * 8);
        if (r8d != 0) {
            eax = 0;
            eax = ((r8d == 2) ? r12d : eax);
            edi |= eax;
        } else {
            edi |= 0xADDR;
        }
        r8d = *(u32*)(rbx);
        while (1) {
            ecx = (r8d | 0xADDR);
            edx = ecx;
            *(u32*)(rbx) = ecx;
            if (var_48 != 0 && (cl & 2) == 0) {
                eax = ecx;
                eax ^= edi;
                eax &= 0xADDR;
                eax ^= ecx;
                ecx = eax;
                ecx ^= edi;
                ecx &= 0xADDR;
                ecx ^= eax;
                ecx |= 1;
                edx = ecx;
                edx ^= edi;
                edx &= 0xADDR;
                edx ^= ecx;
                edx |= 2;
                *(u32*)(rbx) = edx;
            }
            if ((r8b & 4) == 0) {
                eax = edi;
                eax ^= edx;
                eax &= 0xADDR;
                edx ^= eax;
                edx |= 4;
                *(u32*)(rbx) = edx;
            }
            eax = r8d;
            __asm("lock cmpxchg [rsi], edx");
            if (*(u32*)(rsi) == eax) break;
            r8d = eax;
            continue;
        }
        if ((r8b & 4) == 0) {
            sub_14000846c(rsi, 1, ebp);
        }
        if ((*(u8*)(rbx) & 2) == 0) {
            eax = *(u32*)(rbx);
            eax ^= edi;
            eax &= 0xADDR;
            eax ^= *(u32*)(rbx);
            ecx = eax;
            ecx ^= edi;
            ecx &= r12d;
            ecx ^= eax;
            ecx |= 1;
            eax = ecx;
            eax ^= edi;
            eax &= 0xADDR;
            eax ^= ecx;
            *(u32*)(rbx) = eax;
        }
    }
    rax = rbx;
    return;
}
