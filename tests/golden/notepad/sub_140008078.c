// L1 structure recovery (16 blocks)
void sub_140008078_l1(void) {
    rbp = r9;
    r14 = r8;
    rdi = rdx;
    rbx = rcx;
    if (r9 < 0xADDR) {
        sub_140009200(var_38, 0xADDR, "onecore\\internal\\sdk\\inc\\wil/Staging.h");
    }
    rsi = *(u64*)((rcx + 0xADDR));
    r15d = 0;
    *(u64*)((rcx + 0xADDR)) = 0;
    if (rsi != 0) {
        GetProcessHeap();
        HeapFree(rax, 0, rsi);
    }
    *(u64*)((rbx + 0xADDR)) = rdi;
    *(u64*)((rbx + 0xADDR)) = (rdi + r14);
    rax = (rdi + rbp);
    *(u64*)((rbx + 0xADDR)) = rax;
    *(u8*)((rbx + 0xADDR)) = r15b;
    if (r14 < 0xADDR) {
        *(u16*)(rdi) = r15w;
        *(u16*)((rdi + 2)) = ax;
        *(u16*)((rdi + 4)) = ax;
        *(u8*)((rdi + 8)) = *(u8*)((rbx + 4));
        eax = (uint32)(*(u16*)((rbx + 6)));
        *(u16*)((rdi + 6)) = ax;
        al = *(u8*)((rbx + 8));
        *(u8*)((rdi + 9)) = al;
        rax = (*(u64*)((rbx + 0xADDR)) + 0xADDR);
        *(u64*)((rbx + 0xADDR)) = rax;
    } else {
        rcx = (rdi + 2);
        if (*(u16*)(rdi) > r15w) {
            al = 1;
        } else {
            eax = (uint32)(*(u16*)(rbx));
            al = r15b;
        }
        *(u8*)((rbx + 0xADDR)) = al;
        if (*(u16*)(rdi) != r15w) {
            *(u16*)(rdi) = r15w;
            *(u16*)((rdi + 2)) = ax;
            *(u16*)((rdi + 4)) = ax;
            *(u8*)((rdi + 8)) = *(u8*)((rbx + 4));
            eax = (uint32)(*(u16*)((rbx + 6)));
            *(u16*)((rdi + 6)) = ax;
            al = *(u8*)((rbx + 8));
            *(u8*)((rdi + 9)) = al;
            rax = (*(u64*)((rbx + 0xADDR)) + 0xADDR);
            *(u64*)((rbx + 0xADDR)) = rax;
            return;
        }
        eax = (uint32)(*(u16*)(rbx));
        if (*(u16*)(rcx) != ax) {
            *(u16*)(rdi) = r15w;
            *(u16*)((rdi + 2)) = ax;
            *(u16*)((rdi + 4)) = ax;
            *(u8*)((rdi + 8)) = *(u8*)((rbx + 4));
            eax = (uint32)(*(u16*)((rbx + 6)));
            *(u16*)((rdi + 6)) = ax;
            al = *(u8*)((rbx + 8));
            *(u8*)((rdi + 9)) = al;
            rax = (*(u64*)((rbx + 0xADDR)) + 0xADDR);
            *(u64*)((rbx + 0xADDR)) = rax;
            return;
        }
        eax = (uint32)(*(u16*)((rbx + 2)));
        if (*(u16*)((rdi + 4)) != ax) {
            *(u16*)(rdi) = r15w;
            *(u16*)((rdi + 2)) = ax;
            *(u16*)((rdi + 4)) = ax;
            *(u8*)((rdi + 8)) = *(u8*)((rbx + 4));
            eax = (uint32)(*(u16*)((rbx + 6)));
            *(u16*)((rdi + 6)) = ax;
            al = *(u8*)((rbx + 8));
            *(u8*)((rdi + 9)) = al;
            rax = (*(u64*)((rbx + 0xADDR)) + 0xADDR);
            *(u64*)((rbx + 0xADDR)) = rax;
            return;
        }
        al = *(u8*)((rbx + 4));
        if (*(u8*)((rdi + 8)) != al) {
            *(u16*)(rdi) = r15w;
            *(u16*)((rdi + 2)) = ax;
            *(u16*)((rdi + 4)) = ax;
            *(u8*)((rdi + 8)) = *(u8*)((rbx + 4));
            eax = (uint32)(*(u16*)((rbx + 6)));
            *(u16*)((rdi + 6)) = ax;
            al = *(u8*)((rbx + 8));
            *(u8*)((rdi + 9)) = al;
            rax = (*(u64*)((rbx + 0xADDR)) + 0xADDR);
            *(u64*)((rbx + 0xADDR)) = rax;
            return;
        }
        eax = (uint32)(*(u16*)((rbx + 6)));
        if (*(u16*)((rdi + 6)) != ax) {
            *(u16*)(rdi) = r15w;
            *(u16*)((rdi + 2)) = ax;
            *(u16*)((rdi + 4)) = ax;
            *(u8*)((rdi + 8)) = *(u8*)((rbx + 4));
            eax = (uint32)(*(u16*)((rbx + 6)));
            *(u16*)((rdi + 6)) = ax;
            al = *(u8*)((rbx + 8));
            *(u8*)((rdi + 9)) = al;
            rax = (*(u64*)((rbx + 0xADDR)) + 0xADDR);
            *(u64*)((rbx + 0xADDR)) = rax;
            return;
        }
        al = *(u8*)((rbx + 8));
        if (*(u8*)((rdi + 9)) != al) {
            *(u16*)(rdi) = r15w;
            *(u16*)((rdi + 2)) = ax;
            *(u16*)((rdi + 4)) = ax;
            *(u8*)((rdi + 8)) = *(u8*)((rbx + 4));
            eax = (uint32)(*(u16*)((rbx + 6)));
            *(u16*)((rdi + 6)) = ax;
            al = *(u8*)((rbx + 8));
            *(u8*)((rdi + 9)) = al;
            rax = (*(u64*)((rbx + 0xADDR)) + 0xADDR);
            *(u64*)((rbx + 0xADDR)) = rax;
            return;
        }
    }
    return;
}
