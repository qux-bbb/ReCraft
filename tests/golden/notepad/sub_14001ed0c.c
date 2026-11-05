// L1 structure recovery (26 blocks)
void sub_14001ed0c_l1(void) {
    ebx = var_b0;
    ebp = r8d;
    rdi = rcx;
    rdx = rcx;
    r14d = r9d;
    sub_14001f5f0((rax - 0xADDR), rcx, ebx);
    esi = 1;
    r15d = 0xADDR;
    xmm1 = *(__m128*)(rax);
    xmm0 = *(f64*)((rax + 0xADDR));
    rax = qword_ADDR;
    var_50 = xmm0;
    var_30 = xmm1;
    if (rax != 0) {
        if (ebx == 0) {
            r8d = 1;
            edx = ebx;
            ecx = 0xADDR;
            sub_140028010(r15d, ebx, esi);
            xmm1 = var_30;
        } else {
            ecx = (rbx - 0xADDR);
        }
    }
    eax = xmm1;
    if (eax != 0) {
        r8 = rdi;
        sub_14001e848(off_140034c38, rdx);
        xmm1 = var_30;
    }
    xmm0 = (xmm1 >> 4);
    r8d = xmm0;
    if (r8d != 0) {
        rax = qword_ADDR;
        if (rax != 0) {
            xmm1 >>= 8;
            r9d = 0;
            edx = xmm1;
            ecx = 0xADDR;
            sub_140028010(r15d, xmm1, r8, 0);
        } else {
            rax = qword_ADDR;
        }
    }
    edi = var_50;
    if (edi == 0) {
        eax = dword_ADDR;
        if (eax != 0) {
            AcquireSRWLockExclusive(off_140034c40);
            if (qword_ADDR == 0) {
                qword_ADDR = 0;
                rax = qword_ADDR;
                if (rax != 0) {
                    r8 = -1;
                    sub_140028010(off_140034ca0, sub_140012540);
                } else {
                    rax = qword_ADDR;
                }
            }
            ReleaseSRWLockExclusive(off_140034c40);
        }
    }
    if (ebp != 0) {
        rax = qword_ADDR;
        edx = (ebx | 0xADDR);
        edx = ((r14d == 0) ? ebx : edx);
        if (rax != 0) {
            r9d = 0;
            r8d = 0;
            ecx = 0xADDR;
            sub_140028010(r15d, rdx, 0, 0);
        } else {
            rax = qword_ADDR;
        }
    }
    if (edi != 0) {
    } else {
        rax = qword_ADDR;
        if (rax != 0) {
            sub_140028010(r15d, ebx, var_c8);
        }
    }
    r11 = &var_70;
    eax = 0;
    return;
}
