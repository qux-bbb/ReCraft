// L1 structure recovery (4 blocks)
void libc_probe_l1(void) {
    __x86.get_pc_thunk.bx();
    ebx += 0xADDR;
    var_0 = 0xADDR;
    malloc(0xADDR);
    if (eax == 0) {
        eax = -1;
    } else {
        *(u32*)(-1) = 0xADDR;
        var_0 = -1;
        puts(eax);
        var_0 = -1;
        free(eax);
        eax = 3;
    }
    return;
}
