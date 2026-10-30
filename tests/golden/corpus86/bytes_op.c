// L1 structure recovery (13 blocks)
void bytes_op_l1(void) {
    var_c = ebx;
    ebx = var_18;
    eax = 0;
    var_8 = ebp;
    ebp = var_14;
    var_4 = esi;
    var_0 = edi;
    edi = 0;
    if (ebx > 0 /* signed */) {
        if (ebx >= 8) {
            ecx = ebx;
            xmm3 = 0;
            ecx &= -0xADDR;
            xmm2 = 0;
            do {
                xmm1 = *(u32*)((eax + ebp));
                xmm0 = 0;
                __asm("punpcklbw xmm1, xmm0");
                __asm("punpcklwd xmm1, xmm0");
                xmm3 += xmm1;
                xmm1 = *(u32*)(((eax + ebp) + 4));
                __asm("punpcklbw xmm1, xmm0");
                eax += 8;
                __asm("punpcklwd xmm1, xmm0");
                xmm2 += xmm1;
            } while (eax < ecx /* signed */);
            xmm2 += xmm3;
            xmm0 = xmm2;
            xmm0 >>= 8;
            xmm2 += xmm0;
            xmm0 = xmm2;
            xmm0 >>= 4;
            xmm2 += xmm0;
            edi = xmm2;
        }
    }
    edx = 0;
    esi = 0;
    if (eax >= ebx /* signed */) {
        eax = edi;
        edi = var_0;
        esi = var_4;
        ebp = var_8;
        ebx = var_c;
        return;
    }
    ecx = ebx;
    ecx -= eax;
    if (ecx < 2 /* signed */) {
        ecx = (uint32)(*(u8*)((eax + ebp)));
        edi += ecx;
    } else {
        ecx = (ebx + 0xADDR);
        var_18 = ecx;
        do {
            ecx = (uint32)(*(u8*)((eax + ebp)));
            edx += ecx;
            ecx = (uint32)(*(u8*)(((eax + ebp) + 1)));
            eax += 2;
            esi += ecx;
            ecx = eax;
        } while (eax < var_18 /* signed */);
        if (ecx < ebx /* signed */) {
            ecx = (uint32)(*(u8*)((eax + ebp)));
            edi += ecx;
            eax = (esi + edx);
            eax += edi;
            edi = var_0;
            esi = var_4;
            ebp = var_8;
            ebx = var_c;
            return;
        }
    }
    eax = (esi + edx);
    eax += edi;
    edi = var_0;
    esi = var_4;
    ebp = var_8;
    ebx = var_c;
    return;
}
