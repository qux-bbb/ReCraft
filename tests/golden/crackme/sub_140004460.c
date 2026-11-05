// L1 structure recovery (14 blocks)
void sub_140004460_l1(void) {
    eax = dword_ADDR;
    ebx = 0;
    edi = 3;
    if (eax != 0) {
        if (eax < edi /* signed */) {
            eax = edi;
            dword_ADDR = eax;
        }
    } else {
        dword_ADDR = 0xADDR;
    }
    sub_140008e60((int64)(eax), 8);
    ecx = 0;
    qword_ADDR = rax;
    sub_140008ee0(0);
    if (qword_ADDR != rbx) {
        rbp = rbx;
        rsi = off_1400230b8;
        r14 = off_1400230a0;
        do {
            r8d = 0;
            sub_1400092bc((r14 + 0xADDR), 0xADDR);
            rax = qword_ADDR;
            rdx = (rbp >> 6);
            *(u64*)((rbx + rax)) = r14;
            rax = rbp;
            eax &= 0xADDR;
            rcx = (rax + (rax * 8));
            rax = *(u64*)((off_140023f50 + (rdx * 8)));
            rcx = (*(u64*)(((rax + (rcx * 8)) + 0xADDR)) + 2);
            if (rcx <= 2) {
                *(u32*)(rsi) = 0xADDR;
            }
            rbp += 1;
            r14 += 0xADDR;
            rbx += 8;
            rsi += 0xADDR;
            rdi -= 1;
        } while (rdi != 0);
    } else {
        dword_ADDR = edi;
        sub_140008e60(rdi, 8);
        ecx = 0;
        qword_ADDR = 0;
        sub_140008ee0(0);
        if (qword_ADDR == rbx) {
            eax = -1;
        }
    }
    return;
}
