// L1 structure recovery (3 blocks)
void recursive_l1(void) {
    ebx = ecx;
    if (ecx > 1 /* signed */) {
        ecx -= 1;
        recursive();
        eax *= ebx;
        rbx = *(u64*)(rsp);
        return;
    }
    eax = 1;
    rbx = *(u64*)(rsp);
    return;
}
