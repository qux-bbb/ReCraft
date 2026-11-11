// L1 structure recovery (5 blocks)
void __do_global_dtors_aux_l1(void) {
    var_18 = ebp;
    var_14 = ebx;
    __x86.get_pc_thunk.bx();
    ebx += 0xADDR;
    if (*(u8*)((ebx + 0xADDR)) == 0) {
        if (dword_ADDR[ebx] != 0) {
            var_0 = *(u32*)((ebx + 0xADDR));
            __cxa_finalize(*(u32*)((ebx + 0xADDR)));
        }
        deregister_tm_clones();
        *(u8*)((ebx + 0xADDR)) = 1;
    }
    ebx = var_100000014;
    __asm("leave");
    return;
}
