// L1 structure recovery (5 blocks)
void __do_global_dtors_aux_l1(void) {
    if (*(u8*)(0xADDR) != 0) {
        return;
    }
    if (*(u64*)(0xADDR) != 0) {
        __cxa_finalize(*(u64*)(0xADDR));
    }
    deregister_tm_clones();
    *(u8*)(0xADDR) = 1;
    return;
}
