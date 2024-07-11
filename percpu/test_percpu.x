CPU_NUM = 4;

SECTIONS
{
    . = ALIGN(4K);
    _percpu_start = .;
    .percpu 0x0 (NOLOAD) : AT(_percpu_start) {
        _percpu_load_start = .;
        *(.percpu .percpu.*)
        _percpu_load_end = .;
        . = _percpu_load_start + ALIGN(64) * CPU_NUM;
    }
    . = _percpu_start + SIZEOF(.percpu);
}
INSERT AFTER .bss;
