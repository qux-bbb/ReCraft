// L1 structure recovery (39 blocks)
void sub_14000540c_l1(void) {
    r8d = *(u32*)((rcx + 0xADDR));
    bpl = dl;
    rbx = rcx;
    r14d = 8;
    if (r8d > 5 /* signed */) {
        r8d -= 6;
        if (r8d == 0) {
            edx = *(u32*)((rcx + 0xADDR));
            rdi = r14;
            rcx = *(u64*)((rcx + 0xADDR));
            eax = (edx >> 4);
            rax = (rcx + 8);
            *(u64*)((rbx + 0xADDR)) = rax;
            rsi = *(u64*)(rcx);
            loc_ADDR:
            eax = (edx >> 4);
            if ((al & 1) != 0 && rsi < 0 /* signed */) {
                rsi = -(rsi);
                edx |= 0xADDR;
                *(u32*)((rbx + 0xADDR)) = edx;
            }
            if (*(u32*)((rbx + 0xADDR)) >= 0 /* signed */) {
                r8 = *(u64*)((rbx + 8));
                edx &= -0xADDR;
                *(u32*)((rbx + 0xADDR)) = edx;
                sub_140004eac((rbx + 0xADDR), (int64)(*(u32*)((rbx + 0xADDR))), *(u64*)((rbx + 8)));
            } else {
                *(u32*)((rbx + 0xADDR)) = 1;
            }
            if (rsi == 0) {
                *(u32*)((rbx + 0xADDR)) &= -0xADDR;
            }
            *(u8*)((rbx + 0xADDR)) = 0;
            r8b = bpl;
            rcx = rbx;
            if (rdi != r14) {
                sub_140005884(rbx, esi, r8b);
            } else {
                sub_140005a24(rbx, rsi, r8b);
            }
            eax = (*(u32*)((rbx + 0xADDR)) >> 7);
            if ((al & 1) != 0) {
                if (*(u32*)((rbx + 0xADDR)) == 0) {
                    *(u64*)((rbx + 0xADDR)) -= 1;
                    rcx = *(u64*)((rbx + 0xADDR));
                    *(u8*)(rcx) = 0xADDR;
                    *(u32*)((rbx + 0xADDR)) += 1;
                } else {
                    rax = *(u64*)((rbx + 0xADDR));
                    if (*(u8*)(rax) != 0xADDR) {
                        *(u64*)((rbx + 0xADDR)) -= 1;
                        rcx = *(u64*)((rbx + 0xADDR));
                        *(u8*)(rcx) = 0xADDR;
                        *(u32*)((rbx + 0xADDR)) += 1;
                        al = 1;
                        return;
                    }
                }
            }
            al = 1;
        } else {
            r8d -= 1;
            if (r8d != 0) {
                r8d -= 2;
                if (r8d != 0) {
                    r8d -= 1;
                    if (r8d == 0) {
                        edx = *(u32*)((rcx + 0xADDR));
                        edi = 4;
                        rcx = *(u64*)((rcx + 0xADDR));
                        eax = (edx >> 4);
                        rax = (rcx + 8);
                        *(u64*)((rbx + 0xADDR)) = rax;
                        if ((al & 1) == 0) goto loc_ADDR;
                    } else if (r8d != 1) {
                        rax = *(u64*)((rcx + 8));
                        r9d = 0;
                        r8d = 0;
                        edx = 0;
                        *(u8*)((rax + 0xADDR)) = 1;
                        *(u32*)((rax + 0xADDR)) = 0xADDR;
                        rax = *(u64*)((rcx + 8));
                        ecx = 0;
                        var_28 = rax;
                        var_20 = 0;
                        sub_140008bd0(0, 0, 0, 0, 0, *(u64*)((rcx + 8)));
                        al = 0;
                    }
                }
            }
        }
    } else if (r8d != 5) {
        if (r8d != 0) {
            r8d -= 1;
            if (r8d == 0) {
                edx = *(u32*)((rcx + 0xADDR));
                edi = 1;
                rcx = *(u64*)((rcx + 0xADDR));
                eax = (edx >> 4);
                rax = (rcx + 8);
                *(u64*)((rbx + 0xADDR)) = rax;
                if ((dil & al) == 0) goto loc_ADDR;
            } else {
                r8d -= 1;
                if (r8d == 0) {
                    edx = *(u32*)((rcx + 0xADDR));
                    edi = 2;
                    rcx = *(u64*)((rcx + 0xADDR));
                    eax = (edx >> 4);
                    rax = (rcx + 8);
                    *(u64*)((rbx + 0xADDR)) = rax;
                    if ((al & 1) == 0) goto loc_ADDR;
                }
            }
        }
    }
    return;
    loc_ADDR:
    esi = (uint32)(*(u16*)(rcx));
    goto loc_ADDR;
    rsi = (int64)(*(u8*)(rcx));
    goto loc_ADDR;
    loc_ADDR:
    esi = (uint32)(*(u8*)(rcx));
    goto loc_ADDR;
    rsi = (int64)(*(u32*)(rcx));
    goto loc_ADDR;
    loc_ADDR:
    esi = *(u32*)(rcx);
    goto loc_ADDR;
}
