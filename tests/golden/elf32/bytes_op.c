// L1 structure recovery (5 blocks)
void bytes_op_l1(void) {
    var_0 = ebx;
    ebx = var_c;
    if (ebx <= 0 /* signed */) {
        ebx = var_0;
        eax = 0;
        return;
    }
    eax = var_8;
    edx = 0;
    ebx += eax;
    do {
        ecx = (uint32)(*(u8*)(eax));
        eax += 1;
        edx += ecx;
    } while (eax != ebx);
    eax = edx;
    ebx = var_0;
    return;
}
