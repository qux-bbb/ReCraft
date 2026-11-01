// L1 structure recovery (9 blocks)
void sub_140005e90_l1(void) {
    edi = (uint32)(r8b);
    rax = off_1400354d8;
    esi = edx;
    rbx = off_1400353f0;
    rbp = rcx;
    rbx = ((r9d != 0) ? rax : rbx);
    eax = *(u32*)(rbx);
    if ((al & 2) == 0) {
        var_68 = 1;
        sub_140004d94();
        rcx = var_70;
        r14d = eax;
        var_28 = rcx;
        rax = &var_68;
        var_20 = rax;
        sub_140009798(rbp, esi, edi);
        ecx = var_68;
        dil = (uint8)((eax != 0));
        ecx = -(ecx);
        edx = (((ecx != 0) ? 1 : 0) + 6);
        __asm("xchg [rbx], edx");
        if (var_68 == 0 && (dl & 4) == 0) {
            edx = 0;
            sub_14000846c(rbx, 0, r14d);
        }
        al = dil;
    } else if ((al & 1) != 0) {
        rax = var_70;
        var_28 = rax;
        var_20 = 0;
        sub_140009798(rcx, rdx, edi);
        al = (uint8)((eax != 0));
    } else {
        al = 0;
    }
    return;
}
