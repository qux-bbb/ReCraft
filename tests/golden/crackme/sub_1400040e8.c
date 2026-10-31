// L1 structure recovery (12 blocks)
void sub_1400040e8_l1(void) {
    var_60 = rcx;
    r14 = rdx;
    rsi = rcx;
    if (rdx == 0) {
        sub_140008860();
    }
    r12b = 0;
    ebp = 0;
    if (*(u32*)(rdx) > ebp /* signed */) {
        do {
            rax = *(u64*)((rsi + 0xADDR));
            rdi = (int64)(*(u32*)((rax + 0xADDR)));
            sub_140002b44(rcx);
            rax += 4;
            rdi += rax;
            rax = *(u64*)((rsi + 0xADDR));
            var_68 = rdi;
            rbx = (int64)(*(u32*)((rax + 0xADDR)));
            sub_140002b44(rcx);
            r15d = *(u32*)((rax + rbx));
            if (r15d > 0 /* signed */) {
                rax = (int64)(ebp);
                r13 = (rax + (rax * 4));
                while (1) {
                    rsi = (int64)(*(u32*)(rdi));
                    sub_140002b44(rcx);
                    rbx = (int64)(*(u32*)((r14 + 4)));
                    rsi += rax;
                    rax = var_60;
                    rdi = *(u64*)((rax + 0xADDR));
                    sub_140002b30(rcx);
                    rcx = (rax + (r13 * 4));
                    rcx += rbx;
                    sub_1400037fc(rcx, rsi, rdi);
                    if (eax != 0) break;
                    rdi = var_68;
                    r15d -= 1;
                    rdi += 4;
                    var_68 = rdi;
                    if (r15d > 0 /* signed */) continue;
                }
                r12b = 1;
                rsi = var_60;
            }
            ebp += 1;
        } while (ebp < *(u32*)(r14) /* signed */);
            rsi = var_60;
    }
    al = r12b;
    return;
}
