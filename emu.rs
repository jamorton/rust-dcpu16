
// dcpu-16 emulator, for fun and learnings.
// See: dcpu-16 specification; 0x10c.com/doc/dcpu-16.txt

type cpu_state = {
    regs:   [mut u16],
    mut pc: u16,
    mut sp: u16,
    mut o:  u16,
    mem:    [mut u16],
    mut cycles: uint
};

// Basic instruction
// format: bbbbbbaaaaaaoooo
enum basic_opcode {
    NBI,     // non-basic instruction
    SET,     // a = b
    ADD,     // a = a + b
    SUB,     // a = a - b
    MUL,     // a = a * b
    DIV,     // a = a / b
    MOD,     // a = a % b
    SHL,     // a = a << b
    SHR,     // a = a >> b
    AND,     // a = a & b
    BOR,     // a = a | b
    XOR,     // a = a ^ b
    IFE,     // skip next instruction if a != b
    IFN,     // skip next instruction if a == b
    IFG,     // skip next instruction if a <= b
    IFB      // skip next instruction if (a & b) == 0
}

// Non basic instruction
// format: aaaaaaoooooo0000
enum special_opcode {
    JSR = 1 // push pc to stack, set pc to a
}

// Ew.
fn basic_opcode(v: uint) -> basic_opcode {
    unsafe { unsafe::reinterpret_cast(v) }
}

fn special_opcode(v: uint) -> special_opcode {
    unsafe { unsafe::reinterpret_cast(v) }
}

fn new_cpu_state() -> cpu_state {
    {
              regs: vec::to_mut(vec::from_elem(8u, 0u16)),
        mut     pc: 0u16,
        mut     sp: 0u16,
        mut      o: 0u16,
               mem: vec::to_mut(vec::from_elem(0x10000u, 0u16)),
        mut cycles: 0u
    }
}

fn error(out: str) {
    io::print("dcpu16: emu error: ");
    io::println(out);
}

fn next_pc(cpu: cpu_state) -> u16 {
    let val = cpu.mem[cpu.sp];
    cpu.sp += 1u16;
    val
}

fn get_val(cpu: cpu_state, key: u16) -> u16 {
    let k = key as int;

    cpu.cycles += alt k {
      0x10 to 0x17 | 0x1E | 0x1F { 1u }
      _ { 0u }
    };

    alt k {
      0x00 to 0x07 { cpu.regs[k] }
      0x08 to 0x0F { cpu.mem[cpu.regs[k - 0x8]] }
      0x10 to 0x17 { cpu.mem[cpu.regs[k - 0x10] + next_pc(cpu)] }
      0x18 { let r = cpu.mem[cpu.sp]; cpu.sp += 1u16; r }
      0x19 { cpu.mem[cpu.sp] }
      0x1a { cpu.sp -= 1u16; cpu.mem[cpu.sp] }
      0x1b { cpu.sp }
      0x1c { cpu.pc }
      0x1d { cpu.o  }
      0x1e { cpu.mem[next_pc(cpu)] }
      0x1f { next_pc(cpu) }
      0x20 to 0x3f { key - 0x20u16 }
      _ { error("get_val: invalid value"); 0u16 }
    }
}

fn set_val(cpu: cpu_state, key: u16, v: u16) {
    let k = key as int;
    alt k {
      0x00 to 0x07 { cpu.regs[k] = v; }
      0x08 to 0x0f { cpu.mem[cpu.regs[k - 0x08]] = v; }
      0x10 to 0x17 { cpu.mem[cpu.regs[k - 0x10] + next_pc(cpu)] = v; }
      0x18 { cpu.mem[cpu.sp] = v; cpu.sp += 1u16; }
      0x19 { cpu.mem[cpu.sp] = v; }
      0x1a { cpu.sp -= 1u16; cpu.mem[cpu.sp] = v; }
      0x1b { cpu.sp = v; }
      0x1c { cpu.pc = v; }
      0x1d { cpu.o = v; }
      0x1e to 0x3f { error("set_val: attempt to set to a literal"); }
      _ { error("set_val: invalid value"); }
    }
}

fn step(cpu: cpu_state) {

    let word = next_pc(cpu);
    let op   = basic_opcode((word & 0b0000000000001111u16) as uint);
    let ak   =              (word & 0b0000001111110000u16) >> 4u16;
    let bk   =              (word & 0b1111110000000000u16) >> 10u16;
    let a    = get_val(cpu, ak) as uint;
    let b    = get_val(cpu, bk) as uint;

    // Non-basic instructions
    if op == NBI {
        alt special_opcode(a) {
          JSR { // JSR
            cpu.sp -= 1u16;
            cpu.mem[cpu.sp] = next_pc(cpu);
            cpu.pc = b as u16;
            cpu.cycles += 1u
          }
          _ { error("invalid non-basic instruction"); }
        }
        cpu.pc += 1u16;
    } else {
        // Basic instructions
        let mut res = 0u;
        cpu.cycles += alt op {
          SET { res = b; 1u }
          ADD { res = a + b; 2u }
          SUB { res = a - b; 2u }
          MUL { res = a * b; 3u }
          DIV { res = if (b == 0u) { 0u } else { a / b }; 3u }
          MOD { res = if (b == 0u) { 0u } else { a % b }; 2u }
          SHL { res = a << b; 2u }
          SHR { res = a >> b; 2u }
          AND { res = a & b; 1u }
          BOR { res = a | b; 1u }
          XOR { res = a ^ b; 1u }
          IFE { res = (a == b) as uint; 2u }
          IFN { res = (a != b) as uint; 2u }
          IFG { res = (a > b) as uint; 2u }
          IFB { res = ((a & b) != 0u) as uint; 2u }
          _   { error("Invalid basic instruction"); 0u }
        };

        // special logic
        alt op {
          ADD | SUB | MUL | DIV | SHL | SHR  {
            cpu.o = (res >> 16) as u16;
            set_val(cpu, ak, (res & 0xFFFFu) as u16);
          }
          SET | MOD |  AND | BOR | XOR {
            set_val(cpu, ak, (res & 0xFFFFu) as u16);
          }
          IFE | IFN | IFG | IFB {
            cpu.pc += 1u16 - (res as u16); // if res is true (1) we don't skip
          }
          NBI { }
        }
    }
}

fn dump_header() {
    io::println(" A     B     C     X     Y     Z     I     J     PC    SP    O");
    io::println("----------------------------------------------------------------");
}

fn dump_state(cpu: cpu_state) {
    io::println(#fmt(
        "%04x  %04x  %04x  %04x  %04x  %04x  %04x  %04x  %04x  %04x  %04x",
        cpu.regs[0] as uint, cpu.regs[1] as uint, cpu.regs[2] as uint,
        cpu.regs[3] as uint, cpu.regs[4] as uint, cpu.regs[5] as uint,
        cpu.regs[6] as uint, cpu.regs[7] as uint, cpu.pc as uint,
        cpu.sp as uint, cpu.o as uint
    ));
}

fn main() {
    dump_header();
    
    let cpu = new_cpu_state();
    cpu.regs[0] = 0xFFFFu16;
    dump_state(cpu);
}
