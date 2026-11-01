// L1 structure recovery (26 blocks)
void sub_1400068f0_l1(void) {
    al = *(u8*)((rcx + 2));
    r15d = 0;
    r9 = *(u64*)(rdx);
    rbp = r8;
    r14 = rdx;
    rbx = rcx;
    r12d = (r15 + 0xADDR);
    if (al != 1) {
        if (al != 2) {
            eax = (uint32)(*(u16*)(rbx));
            rdi = (rbx + 8);
            *(u16*)(rdi) = ax;
            if (ax != 0) {
                ecx = (uint32)(*(u16*)(rdi));
                rcx += r9;
                if (rcx <= rbp) {
                    *(u64*)((rbx + 0xADDR)) = r9;
                    al = 1;
                    *(u64*)(r14) = rcx;
                } else {
                    al = 0;
                }
            } else {
                rsi = (r9 + 2);
                if (rsi > rbp) {
                    al = 0;
                    return;
                }
                if (rdi == 0) {
                    _o__errno();
                    *(u32*)(rax) = r12d;
                    sub_140002934();
                } else if (r9 == 0) {
                    eax = 0;
                    *(u16*)(rdi) = ax;
                } else {
                    eax = (uint32)(*(u16*)(r9));
                    *(u16*)(rdi) = ax;
                }
                r9 = rsi;
            }
        } else {
            rdi = (r9 + 4);
            if (rdi > rbp) {
                al = 0;
                return;
            }
            *(u64*)((rcx + 0xADDR)) = r9;
            rcx += 4;
            if (rcx == 0) {
                _o__errno();
                *(u32*)(rax) = r12d;
                sub_140002934();
            } else if (r9 == 0) {
                *(u32*)(rcx) = 0;
            } else {
                *(u32*)(rcx) = *(u32*)(r9);
            }
            r9 = rdi;
        }
    } else {
        rdi = (r9 + 2);
        if (rdi > r8) {
            al = 0;
            return;
        }
        *(u64*)((rcx + 0xADDR)) = r9;
        if (r9 == 0) {
            _o__errno();
            *(u32*)(rax) = r12d;
            sub_140002934();
            eax = r15d;
        } else {
            eax = (uint32)(*(u16*)(r9));
        }
        *(u32*)((rbx + 4)) = (uint32)(ax);
        r9 = rdi;
    }
    return;
}
