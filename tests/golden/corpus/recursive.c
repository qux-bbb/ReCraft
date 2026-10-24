// L1 structure recovery (3 blocks)
void recursive_l1(void) {
    var_n8 = rbx;
    ebx = ecx;
    if ((ecx > 1 /* signed */)) {
        ecx = (ecx - 1);
        recursive();
        eax = (eax * ebx);
        rbx = *(u64*)(rsp);
        return;
    }
    eax = 1;
    rbx = *(u64*)(rsp);
    return;
}
