    SET I, 0xa                ; a861
    SET A, 0x2000            ; 7c01 2000
    SET [0x2000+I], [A]      ; 2161 2000
    SUB I, 0x1                 ; 8463
    IFN I, 0x0                 ; 806d
    SET PC, 0x4            ; 7dc1 000d [*]