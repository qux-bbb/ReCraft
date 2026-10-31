// L1 structure recovery (39 blocks)
void sub_140006830_l1(void) {
    *(u32*)((rcx + 0xADDR)) = (*(u32*)((rcx + 0xADDR)) | 0xADDR);
    rbx = rcx;
    eax = *(u32*)((rcx + 0xADDR));
    if (eax >= 0 /* signed */) {
        if (eax == 0) {
            if (*(u8*)((rcx + 0xADDR)) == 0xADDR) {
                *(u32*)((rcx + 0xADDR)) = 1;
                eax = 1;
            }
        }
    } else {
        al = *(u8*)((rcx + 0xADDR));
        al -= 0xADDR;
        al &= 0xADDR;
        al = -(al);
        eax = ((al != 0) ? -0xADDR : 0);
        eax += 0xADDR;
        *(u32*)((rcx + 0xADDR)) = eax;
    }
    r8 = *(u64*)((rcx + 8));
    rdi = (rcx + 0xADDR);
    eax += 0xADDR;
    sub_140004eac(rdi, (int64)(eax), *(u64*)((rcx + 8)));
    rdx = (rbx + 0xADDR);
    r10d = 0xADDR;
    if (al != 0) {
        r8d = *(u32*)((rbx + 0xADDR));
        r9 = rdi;
    } else {
        if (*(u64*)(rdx) != 0) {
            rax = *(u64*)((rbx + 0xADDR));
            rax >>= 1;
        } else {
            eax = r10d;
        }
        *(u32*)((rbx + 0xADDR)) = (rax - 0xADDR);
        r9 = (rbx + 0xADDR);
    }
    rax = *(u64*)((rdi + 0xADDR));
    rax = ((rax == 0) ? rdi : rax);
    *(u64*)((rbx + 0xADDR)) = rax;
    eax = 0;
    rcx = *(u64*)((rbx + 0xADDR));
    rax = (rcx + 8);
    *(u64*)((rbx + 0xADDR)) = rax;
    rdx = *(u64*)(rdx);
    xmm0 = *(f64*)(rcx);
    r11 = *(u64*)((rbx + 8));
    rdi = *(u64*)(rbx);
    esi = (int32)(*(u8*)((rbx + 0xADDR)));
    var_60 = xmm0;
    if (rdx != 0) {
        r9 = *(u64*)((r9 + 0xADDR));
        rcx = *(u64*)((rbx + 0xADDR));
        r10 = *(u64*)((rbx + 0xADDR));
        r9 >>= 1;
        r9 += rdx;
        rcx >>= 1;
        r10 >>= 1;
    } else {
        r9 += r10;
        rdx = (rbx + 0xADDR);
        rcx = r10;
    }
    var_48 = r11;
    var_40 = 1;
    var_38 = rdi;
    var_30 = r8d;
    var_28 = esi;
    var_20 = rcx;
    sub_14000c060(&var_60, rdx, r10);
    eax = *(u32*)((rbx + 0xADDR));
    eax >>= 5;
    if ((al & 1) != 0) {
        if (*(u32*)((rbx + 0xADDR)) == 0) {
            rdi = *(u64*)((rbx + 8));
            if (*(u8*)((rdi + 0xADDR)) == 0) {
                sub_140006cc0(rdi);
            }
            r9 = *(u64*)((rbx + 0xADDR));
            r10 = *(u64*)((rdi + 0xADDR));
            r8d = (uint32)(*(u8*)(r9));
            rax = *(u64*)((r10 + 0xADDR));
            if (*(u8*)((r8 + rax)) != 0xADDR) {
                rax = *(u64*)(r10);
                do {
                    r9 += 1;
                    r8d = (uint32)(*(u8*)(r9));
                } while ((*(u8*)((rax + (r8 * 2))) & 4) != 0);
            }
            rax = *(u64*)((r10 + 0xADDR));
            ecx = (uint32)(r8b);
            if (*(u8*)((rcx + rax)) == 0xADDR) {
                r8b = *(u8*)((r9 + 2));
            }
            rax = *(u64*)((r10 + 0xADDR));
            rdx = (r9 + 2);
            __asm("cmovne rdx, r9");
            rcx = *(u64*)(rax);
            al = *(u8*)(rcx);
            *(u8*)(rdx) = al;
            rdx += 1;
            do {
                al = *(u8*)(rdx);
                cl = r8b;
                *(u8*)(rdx) = r8b;
                rdx += 1;
                r8b = al;
            } while (cl != 0);
        }
    }
    al = *(u8*)((rbx + 0xADDR));
    al -= 0xADDR;
    if ((al & 0xADDR) == 0) {
        eax = *(u32*)((rbx + 0xADDR));
        eax >>= 5;
        if ((al & 1) == 0) {
            rdi = *(u64*)((rbx + 8));
            if (*(u8*)((rdi + 0xADDR)) == 0) {
                sub_140006cc0(rdi);
            }
            sub_140005c74(*(u64*)((rbx + 0xADDR)), (rdi + 0xADDR));
        }
    }
    rcx = *(u64*)((rbx + 0xADDR));
    al = *(u8*)(rcx);
    if (al == 0xADDR) {
        *(u32*)((rbx + 0xADDR)) = (*(u32*)((rbx + 0xADDR)) | 0xADDR);
        rcx += 1;
        *(u64*)((rbx + 0xADDR)) = rcx;
        al = *(u8*)(rcx);
    }
    al -= 0xADDR;
    if (al <= 0xADDR) {
        if (((0xADDR >> rax) & 1) != 0) {
            *(u32*)((rbx + 0xADDR)) = (*(u32*)((rbx + 0xADDR)) & -0xADDR);
            *(u8*)((rbx + 0xADDR)) = 0xADDR;
        }
    }
    rdx = -1;
    do {
        rdx += 1;
    } while (*(u8*)((rcx + rdx)) != 0);
    al = 1;
    *(u32*)((rbx + 0xADDR)) = edx;
    return;
}
