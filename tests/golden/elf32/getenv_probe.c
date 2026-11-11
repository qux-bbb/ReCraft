// L1 structure recovery (3 blocks)
void getenv_probe_l1(void) {
    __x86.get_pc_thunk.bx();
    ebx += 0xADDR;
    eax = (ebx + 0xADDR);
    var_0 = eax;
    getenv(eax);
    edx = 0;
    if (eax != 0) {
        var_0 = eax;
        strlen(eax);
        edx = eax;
    }
    eax = edx;
    return;
}
