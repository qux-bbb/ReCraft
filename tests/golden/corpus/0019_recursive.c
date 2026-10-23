// L1 structure recovery (3 blocks)
void recursive_l1(void) {
    *(u64*)((rsp - 8)) = rbx;
    rsp = (rsp - 8);
    rsp = (rsp - 0xADDR);
    ebx = ecx;
    if ((ecx > 1 /* signed */)) {
        ecx = (ecx - 1);
        recursive();
        eax = (eax * ebx);
        rsp = (rsp + 0xADDR);
        rbx = *(u64*)(rsp);
        rsp = (rsp + 8);
        return;
    }
    eax = 1;
    rsp = (rsp + 0xADDR);
    rbx = *(u64*)(rsp);
    rsp = (rsp + 8);
    return;
}
