// L1 structure recovery (57 blocks)
void sub_140004f4c_l1(void) {
    var_c8 = r9;
    eax = (uint32)(*(u16*)((rcx + 6)));
    rdi = (rdx + 4);
    ebx = 0;
    var_48 = ax;
    al = *(u8*)((rcx + 8));
    r12 = -1;
    xmm0 = 0;
    var_46 = al;
    r14 = r8;
    r15 = rdx;
    esi = (rbx + 0xADDR);
    r13 = rcx;
    var_44 = ebx;
    var_40 = bx;
    var_38 = xmm0;
    if (*(u64*)((rcx + 0xADDR)) <= rbx) {
        esi = ebx;
        if (*(u32*)(rdi) <= ebx) {
            rax = r14;
        } else {
            while (1) {
                var_b0 = r14;
                sub_1400068f0(&var_48, &var_b0, *(u64*)((r13 + 0xADDR)));
                if (al == 0) break;
                eax = (uint32)(var_40);
                rcx = var_d0;
                if (rcx != rax) {
                    eax = (uint32)(var_40);
                    ecx -= eax;
                    r12d = ecx;
                } else {
                    r8 = rcx;
                    sub_1400274a0(var_c8, var_30);
                    r12d = eax;
                }
                if (r12d > 0 /* signed */) {
                    r14 = var_b0;
                    esi += 1;
                    if (esi < *(u32*)(rdi)) continue;
                }
            }
            if (*(u32*)(rdi) != esi) {
                *(u32*)(rdi) = esi;
                if (*(u8*)((r15 + 2)) != 1) {
                    if (*(u8*)((r15 + 2)) == 2) {
                        rax = *(u64*)((r15 + 0xADDR));
                        if (rax == 0) {
                            _o__errno();
                            *(u32*)(rax) = 0xADDR;
                            sub_140002934();
                        } else if (rdi == 0) {
                            *(u32*)(rax) = 0;
                        } else {
                            *(u32*)(rax) = esi;
                        }
                    }
                } else {
                    rax = *(u64*)((r15 + 0xADDR));
                    ecx = (uint32)(si);
                    if (rax != 0) {
                        *(u16*)(rax) = cx;
                    }
                }
            }
            loc_ADDR:
            __asm("if (jne short 0xADDR /* unmodelled flags */) goto 0xADDR");
            dl = var_46;
            if (dl == 0) {
                if (*(u8*)((r13 + 0xADDR)) != bl) {
                    loc_ADDR:
                    bl = 1;
                }
            } else {
                r8d = var_d8;
                r8d += var_44;
                if (var_44 == r8d) {
                    bl = 1;
                    *(u8*)((r13 + 0xADDR)) = 1;
                    eax = 0;
                    return;
                }
                if (dl != 1) goto loc_ADDR;
            }
            *(u8*)((r13 + 0xADDR)) = bl;
            eax = 0;
        }
    } else {
        rax = *(u64*)((rcx + 0xADDR));
        edx = 0;
        rax -= r8;
        __asm("div qword ptr [rcx+0xADDR]");
        rcx = rax;
        eax = *(u32*)(rdi);
        if (rax > rcx && eax != ecx) {
            al = *(u8*)((r15 + 2));
            *(u32*)(rdi) = ecx;
            if (al != 1) {
                if (al == 2) {
                    rax = *(u64*)((r15 + 0xADDR));
                    if (rax == 0) {
                        _o__errno();
                        *(u32*)(rax) = esi;
                        sub_140002934();
                    } else if (rdi == 0) {
                        *(u32*)(rax) = 0;
                    } else {
                        *(u32*)(rax) = ecx;
                    }
                }
            } else {
                rax = *(u64*)((r15 + 0xADDR));
                if (rax != 0) {
                    *(u16*)(rax) = cx;
                }
            }
        }
        esi = *(u32*)(rdi);
        xmm0 = 0;
        rdi = var_d0;
        rax *= *(u64*)((r13 + 0xADDR));
        var_18 = xmm0;
        var_24 = ebx;
        rax += r14;
        var_20 = bx;
        var_50 = rax;
        eax = (uint32)(*(u16*)((r13 + 6)));
        var_28 = ax;
        al = *(u8*)((r13 + 8));
        var_26 = al;
        while (1) {
            while (rsi != 0) {
                r15 = (rsi >> 1);
                rax = r15;
                rax *= *(u64*)((r13 + 0xADDR));
                rax += r14;
                var_b0 = rax;
                sub_1400068f0(&var_28, &var_b0, *(u64*)((r13 + 0xADDR)));
                eax = (uint32)(var_20);
                if (rdi != rax) {
                    eax = (uint32)(var_20);
                    ecx = edi;
                    ecx -= eax;
                } else {
                    r8 = rdi;
                    sub_1400274a0(var_c8, var_10);
                    ecx = eax;
                }
                if (ecx <= 0 /* signed */) goto loc_ADDR;
            }
            r14 = var_b0;
            rax = r12;
            rax -= r15;
            rsi += rax;
        }
        if (r14 >= var_50) {
            rax = r14;
            return;
        }
        var_b0 = r14;
        sub_1400068f0(&var_48, &var_b0, *(u64*)((r13 + 0xADDR)));
        eax = (uint32)(var_40);
        if (rdi != rax) goto loc_ADDR;
    }
    return;
    loc_ADDR:
    rsi = r15;
    r8 = rdi;
    sub_1400274a0(var_c8, var_30);
    r12d = eax;
    goto loc_ADDR;
    loc_ADDR:
    eax = (uint32)(var_40);
    edi -= eax;
    r12d = edi;
    goto loc_ADDR;
    rax = var_38;
    if (rax != 0) {
        *(u16*)(rax) = r8w;
        goto loc_ADDR;
    }
    _o__errno();
    *(u32*)(rax) = 0xADDR;
    sub_140002934();
    goto loc_ADDR;
    loc_ADDR:
    if (dl != 2) goto loc_ADDR;
    rax = var_38;
    if (rax == 0) {
        _o__errno();
        *(u32*)(rax) = 0xADDR;
        sub_140002934();
        *(u8*)((r13 + 0xADDR)) = 1;
        eax = 0;
        return;
    }
    *(u32*)(rax) = r8d;
    goto loc_ADDR;
}
