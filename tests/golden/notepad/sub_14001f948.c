// L1 structure recovery (6 blocks)
void sub_14001f948_l1(void) {
    rcx = qword_ADDR;
    ebp = 0;
    var_48 = ebp;
    var_40 = ebp;
    SendMessageW(qword_ADDR, 0xADDR, &var_48, &var_40);
    SendMessageW(qword_ADDR, 0xADDR, 0, 0);
    rdi = rax;
    if (rax == 0) {
        al = 0;
    } else {
        ecx = var_48;
        if (var_40 <= ecx) {
            al = 0;
            return;
        }
        LocalLock(rax);
        r8 = rax;
        if (rax == 0) {
            al = 0;
            return;
        }
        edx = var_40;
        ecx = (rbp + 0xADDR);
        edx -= var_48;
        eax = var_48;
        ecx = ((edx < ecx) ? edx : ecx);
        rdx = (r8 + (rax * 2));
        rbx = (rcx + rcx);
        r8 = rbx;
        sub_1400274ac(rsi, (r8 + (rax * 2)));
        word_ADDR[rbx] = bp;
        LocalUnlock(rdi);
        al = 1;
    }
    return;
}
