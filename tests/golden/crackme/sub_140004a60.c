// L1 structure recovery (20 blocks)
void sub_140004a60_l1(void) {
    rbx = r8;
    r15d = edx;
    r12 = rcx;
    edi = 0;
    if (rcx != 0) {
        eax = 1;
    } else {
        eax = edi;
    }
    if (eax != 0) {
        if (r15d < 0 /* signed */) {
            sub_140008e30();
            *(u32*)(rax) = 0xADDR;
            sub_140008c88();
            eax = 0;
        } else if (rbx == 0) {
            sub_140008e30();
            *(u32*)(rax) = 0xADDR;
            sub_140008c88();
            eax = 0;
            return;
        } else if (r15d == 0) {
            eax = 0;
            return;
        }
        rcx = rbx;
        sub_1400045f0(rbx);
        sub_140004b38(rbx);
        if (al != 0) {
            rsi = r12;
            var_28 = r12;
            r14d = 1;
            while (1) {
                var_20 = r14d;
                if (r14d == r15d) break;
                sub_14000acf4(rbx);
                var_60 = eax;
                if (eax != -1) {
                    *(u8*)(rsi) = al;
                    rsi += 1;
                    var_28 = rsi;
                    if (al == 0xADDR) break;
                    r14d += 1;
                    continue;
                }
            }
            *(u8*)(rsi) = dil;
            rdi = r12;
        }
        rcx = rbx;
        sub_1400045fc(rbx);
        rax = rdi;
    } else {
        sub_140008e30();
        *(u32*)(rax) = 0xADDR;
        sub_140008c88();
        eax = 0;
        return;
    }
    return;
}
