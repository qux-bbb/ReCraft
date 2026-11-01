// L1 structure recovery (28 blocks)
void sub_140005830_l1(void) {
    rdi = rcx;
    rsi = r9;
    ecx = 0;
    rbx = r8;
    var_20 = rcx;
    r14 = rdx;
    if (rdi == 0) {
        if (r14 != 0) {
            if (rdi != 0) {
                edi -= ecx;
            }
            *(u32*)(r14) = edi;
        }
        if (rbx == 0) {
            al = 1;
        } else {
            GetModuleFileNameA(rcx, &var_30, 0xADDR);
            if (eax != 0) {
                rcx = &var_30;
                rax = -1;
                do {
                    rax += 1;
                } while (*(u8*)((rcx + rax)) != 0);
                rcx = &var_30;
                rcx += rax;
                rax = &var_30;
                while (rcx > rax) {
                    if (*(u8*)((rcx - 1)) == 0xADDR) break;
                    rcx -= 1;
                }
                if (rsi == 0) {
                    al = 1;
                    return;
                }
                if (rsi > 0xADDR) {
                    *(u8*)(rbx) = 0;
                    al = 1;
                    return;
                }
                r8d = 0xADDR;
                r8 -= rsi;
                rcx -= rbx;
                do {
                    rax = (r8 + rsi);
                    if (rax == 0) break;
                    dl = *(u8*)((rcx + rbx));
                    if (dl == 0) break;
                    *(u8*)(rbx) = dl;
                    rbx += 1;
                    rsi -= 1;
                } while (rsi != 0);
                rcx = (rbx - 1);
                rcx = ((rsi != 0) ? rbx : rcx);
                *(u8*)(rcx) = 0;
            } else {
                al = 0;
            }
        }
    } else {
        GetModuleHandleExW(6, rdi, &var_20);
        if (eax != 0) {
            rcx = var_20;
        } else {
            if (r14 == 0) {
                al = 0;
                return;
            }
            *(u32*)(r14) = (*(u32*)(r14) & eax);
            al = 0;
            return;
        }
    }
    return;
}
