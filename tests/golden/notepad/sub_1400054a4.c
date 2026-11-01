// L1 structure recovery (36 blocks)
void sub_1400054a4_l1(void) {
    r15d = 0;
    rbx = r8;
    rdi = rdx;
    r14 = rcx;
    if (rdx != 0 && rcx != 0) {
        rax = qword_ADDR;
        *(u16*)(rcx) = r15w;
        if (rax == 0) {
            ecx = *(u32*)(rbx);
            rsi = off_14002a8ab;
            bpl = 8;
            if (ecx == 0) {
                rsi = "Exception";
            } else {
                ecx -= 1;
                if (ecx == 0) {
                    rax = "ReturnHr";
                    rsi = "ReturnNt";
                    rsi = (((*(u8*)((rbx + 4)) & bpl) == 0) ? rax : rsi);
                } else {
                    ecx -= 1;
                    if (ecx == 0) {
                        rax = "LogHr";
                        rsi = "LogNt";
                        rsi = (((*(u8*)((rbx + 4)) & bpl) == 0) ? rax : rsi);
                        eax = 0;
                        return;
                    } else if (ecx == 1) {
                        rsi = "FailFast";
                    }
                }
            }
            edx = 0;
            sub_1400029dc(&var_40, 0, 0xADDR);
            if ((*(u8*)((rbx + 4)) & bpl) == 0) {
                ebp = *(u32*)((rbx + 8));
                rax = &var_40;
                var_30 = r15;
                var_28 = 0xADDR;
                var_20 = rax;
                FormatMessageW(0xADDR, 0, ebp, 0xADDR, &var_40, 0xADDR, r15);
            } else {
                rax = qword_ADDR;
                ebp = *(u32*)((rbx + 0xADDR));
                if (rax != 0) {
                    sub_140028010(ebp, &var_40, 0xADDR);
                }
            }
            r9 = *(u64*)((rbx + 0xADDR));
            rdi = (r14 + (rdi * 2));
            rax = *(u64*)((rbx + 0xADDR));
            rdx = rdi;
            rcx = *(u64*)((rbx + 0xADDR));
            if (r9 == 0) {
                r9 = rcx;
                var_20 = rax;
                sub_14000627c(r14, rdx, "%hs!%p: ", rcx, rax);
            } else {
                var_30 = rax;
                eax = *(u32*)((rbx + 0xADDR));
                var_28 = rcx;
                var_20 = eax;
                sub_14000627c(r14, rdx, "%hs(%u)\\%hs!%p: ", r9, *(u32*)((rbx + 0xADDR)), rcx, rax);
            }
            r9 = *(u64*)((rbx + 0xADDR));
            r14 = rax;
            if (r9 != 0) {
                sub_14000627c(rax, rdi, "(caller: %p) ", r9);
                r14 = rax;
            }
            GetCurrentThreadId();
            ecx = *(u32*)((rbx + 0xADDR));
            rdx = &var_40;
            var_38 = rdx;
            var_30 = ebp;
            var_28 = eax;
            var_20 = ecx;
            sub_14000627c(r14, rdi, "%hs(%d) tid(%x) %08X %ws", rsi, *(u32*)((rbx + 0xADDR)), eax, ebp, &var_40);
            if (*(u64*)((rbx + 0xADDR)) != r15) {
                sub_14000627c(rax, rdi, " ", r9);
                r9 = *(u64*)((rbx + 0xADDR));
                if (r9 != 0) {
                    sub_14000627c(rax, rdi, "Msg:[%ws] ", r9);
                }
                r9 = *(u64*)((rbx + 0xADDR));
                if (r9 != 0) {
                    sub_14000627c(rax, rdi, "CallContext:[%hs] ", r9);
                }
                rcx = *(u64*)((rbx + 0xADDR));
                rdx = rdi;
                r9 = *(u64*)((rbx + 0xADDR));
                if (rcx == 0) {
                    rcx = rax;
                    if (r9 == 0) {
                        sub_14000627c(rax, rdi, "\n", r9);
                    } else {
                        sub_14000627c(rax, rdi, "[%hs]", r9);
                    }
                } else {
                    var_20 = rcx;
                    sub_14000627c(rax, rdi, "[%hs(%hs)]", r9, rcx);
                }
            }
        } else if (byte_ADDR != r15b) {
            r8 = rdx;
            rdx = rcx;
            rcx = rbx;
            sub_140028010(rbx, rcx, rdx);
        }
    }
    eax = 0;
    return;
}
