// L1 structure recovery (18 blocks)
void sub_1400077b4_l1(void) {
    edi = 0;
    r14d = ecx;
    if (ecx != 0) {
        eax = (rcx - 1);
        if (eax <= 1) {
            sub_14000e09c();
            rbx = off_140023cd0;
            r8d = 0xADDR;
            ecx = 0;
            sub_14000d770(0, rbx);
            rsi = qword_ADDR;
            qword_ADDR = rbx;
            if (rsi == 0) {
                rsi = rbx;
            }
            rax = &var_78;
            var_70 = rdi;
            var_20 = rax;
            r8d = 0;
            var_78 = rdi;
            edx = 0;
            sub_140007594(rsi, 0, 0, &var_70, &var_78);
            r15 = var_70;
            r8d = 1;
            sub_140007754(r15, var_78);
            rbx = rax;
            if (rax != 0) {
                r8 = (rax + (r15 * 8));
                rax = &var_78;
                var_20 = rax;
                sub_140007594(rsi, rbx, (rax + (r15 * 8)), &var_70, &var_78);
                if (r14d != 1) {
                    var_68 = rdi;
                    sub_14000d64c(rbx, &var_68);
                    esi = eax;
                    if (eax == 0) {
                        rdx = var_68;
                        rcx = rdi;
                        rax = rdx;
                        if (*(u64*)(rdx) != rdi) {
                            do {
                                rax += 8;
                                rcx += 1;
                            } while (*(u64*)(rax) != rdi);
                        }
                        dword_ADDR = ecx;
                        ecx = 0;
                        var_68 = rdi;
                        qword_ADDR = rdx;
                        sub_140008ee0(0);
                        rcx = rbx;
                        var_68 = rdi;
                        sub_140008ee0(rcx);
                    } else {
                        sub_140008ee0(var_68);
                        var_68 = rdi;
                        sub_140008ee0(rbx);
                        edi = esi;
                    }
                } else {
                    eax = var_70;
                    eax -= 1;
                    qword_ADDR = rbx;
                    dword_ADDR = eax;
                    ecx = 0;
                    sub_140008ee0(rcx);
                    eax = edi;
                    return;
                }
            } else {
                sub_140008e30();
                *(u32*)(rax) = (rbx + 0xADDR);
            }
        } else {
            sub_140008e30();
            edi = 0xADDR;
            *(u32*)(rax) = edi;
            sub_140008c88();
        }
    }
    eax = edi;
    return;
}
