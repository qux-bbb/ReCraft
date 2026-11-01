// L1 structure recovery (12 blocks)
void sub_14000627c_l1(void) {
    var_78 = r9;
    rdi = rdx;
    r9 = &var_78;
    rdi -= rcx;
    ebp = 0;
    rdi >>= 1;
    rsi = rdx;
    rbx = rcx;
    if (rdi != 0) {
        if (rdi > 0xADDR) {
            *(u16*)(rcx) = bp;
        } else {
            rdi -= 1;
            sub_1400029f4(rcx, rdi, r8, &var_78);
            if (eax < 0 /* signed */) {
                *(u16*)((rbx + (rdi * 2))) = bp;
            } else {
                rax = (int64)(eax);
            }
        }
    }
    if (rsi != rbx) {
        rax = -1;
        do {
            rax += 1;
        } while (*(u16*)((rbx + (rax * 2))) != bp);
        rbx += (rax * 2);
    }
    rax = rbx;
    return;
}
