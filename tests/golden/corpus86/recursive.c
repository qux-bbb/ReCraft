// L1 structure recovery (3 blocks)
void recursive_l1(void) {
    var_4 = esi;
    esi = var_c;
    if (esi > 1 /* signed */) {
        eax = (esi + 0xADDR);
        var_0 = eax;
        recursive();
        eax *= esi;
        esi = var_4;
        return;
    }
    eax = 1;
    esi = var_4;
    return;
}
