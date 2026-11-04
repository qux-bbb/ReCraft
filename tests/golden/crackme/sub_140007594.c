// L1 structure recovery (59 blocks)
void sub_140007594_l1(void) {
    r15 = var_60;
    r12 = r9;
    rbx = r8;
    r14 = rdx;
    rdi = rcx;
    *(u64*)(r15) = 0;
    *(u64*)(r9) = 1;
    if (rdx != 0) {
        *(u64*)(rdx) = rbx;
        r14 += 8;
    }
    bpl = 0;
    while (1) {
        if (*(u8*)(rdi) != 0xADDR) {
            *(u64*)(r15) += 1;
            if (rbx != 0) {
                al = *(u8*)(rdi);
                *(u8*)(rbx) = al;
                rbx += 1;
            }
            esi = (int32)(*(u8*)(rdi));
            rdi += 1;
            sub_14000e454(esi);
            if (eax != 0) {
                *(u64*)(r15) += 1;
                if (rbx != 0) {
                    al = *(u8*)(rdi);
                    *(u8*)(rbx) = al;
                    rbx += 1;
                }
                rdi += 1;
            }
            if (sil == 0) break;
            loc_ADDR:
            if (bpl != 0) continue;
        } else {
            sil = 0xADDR;
            bpl = (uint8)((bpl == 0));
            rdi += 1;
            goto loc_ADDR;
        }
    }
    rdi -= 1;
    sil = 0;
    loc_ADDR:
    while (1) {
        al = *(u8*)(rdi);
        if (al == 0) break;
        while (1) {
            if (al == 0xADDR) {
                rdi += 1;
                al = *(u8*)(rdi);
                continue;
            }
            if (al != 9) break;
        }
        if (al == 0) break;
        if (r14 != 0) {
            *(u64*)(r14) = rbx;
            r14 += 8;
        }
        *(u64*)(r12) += 1;
        loc_ADDR:
        edx = 1;
        eax = 0;
        goto loc_ADDR;
    }
    do {
        rdi += 1;
        eax += 1;
        loc_ADDR:
        cl = *(u8*)(rdi);
    } while (cl == 0xADDR);
    if (cl != 0xADDR) goto loc_ADDR;
    if ((dl & al) == 0) {
        if (sil != 0) {
            if (*(u8*)((rdi + 1)) == cl) {
                rdi += 1;
                goto loc_ADDR;
            }
        }
        edx = 0;
        sil = (uint8)((sil == 0));
    }
    loc_ADDR:
    eax >>= 1;
    goto loc_ADDR;
    do {
        eax -= 1;
        if (rbx != 0) {
            *(u8*)(rbx) = 0xADDR;
            rbx += 1;
        }
        *(u64*)(r15) += 1;
        loc_ADDR:
    } while (eax != 0);
    al = *(u8*)(rdi);
    if (al != 0) {
        if (sil != 0) goto loc_ADDR;
        if (al != 0xADDR) {
            if (al != 9) {
                loc_ADDR:
                if (edx != 0) {
                    if (rbx != 0) {
                        *(u8*)(rbx) = al;
                        rbx += 1;
                    }
                    sub_14000e454((int32)(*(u8*)(rdi)));
                    if (eax != 0) {
                        *(u64*)(r15) += 1;
                        rdi += 1;
                        if (rbx != 0) {
                            al = *(u8*)(rdi);
                            *(u8*)(rbx) = al;
                            rbx += 1;
                        }
                    }
                    *(u64*)(r15) += 1;
                }
                rdi += 1;
                goto loc_ADDR;
            }
        }
    }
    if (rbx != 0) {
        *(u8*)(rbx) = 0;
        rbx += 1;
    }
    *(u64*)(r15) += 1;
    goto loc_ADDR;
    if (r14 != 0) {
        *(u64*)(r14) = 0;
    }
    *(u64*)(r12) += 1;
    return;
}
