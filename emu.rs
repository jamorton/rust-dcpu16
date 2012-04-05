
// dcpu-16 emulator, for fun and learnings.
// See: dcpu-16 specification; 0x10c.com/doc/dcpu-16.txt

type cpu_state = {
    regs:       [mut u16],
    mut pc:     u16,
    mut sp:     u16,
    mut o:      u16,
    mem:        [mut u16],
    mut cycles: uint,
    mut stop:   bool
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
    EXT, // extension, reserved
    JSR  // push pc to stack, set pc to a
}

// Ew.
fn basic_op(v: uint)   -> basic_opcode   {unsafe{unsafe::reinterpret_cast(v)}}
fn special_op(v: uint) -> special_opcode {unsafe{unsafe::reinterpret_cast(v)}}

fn new_cpu_state() -> cpu_state {
    {
              regs: vec::to_mut(vec::from_elem(8u, 0u16)),
        mut     pc: 0u16,
        mut     sp: 0u16,
        mut      o: 0u16,
               mem: vec::to_mut(vec::from_elem(0x10000u, 0u16)),
        mut cycles: 0u,
        mut   stop: false
    }
}

fn error(out: str) {
    io::print("rust-dcpu16 emu error: ");
    io::println(out);
}

fn next_pc(cpu: cpu_state) -> u16 {
    let val = cpu.mem[cpu.pc];
    cpu.pc += 1u16;
    val
}

enum value {
    value_literal(u16),
    value_reg(u16),
    value_mem(u16),
    value_sp,
    value_pc,
    value_o,
}

fn new_value(cpu: cpu_state, key: u16) -> value {
    let k = key as int;

    cpu.cycles += alt k {
      0x10 to 0x17 | 0x1E | 0x1F { 1u }
      _ { 0u }
    };

    alt k {
      0x00 to 0x07 { value_reg(key) }
      0x08 to 0x0F { value_mem(cpu.regs[k - 0x8]) }
      0x10 to 0x17 { value_mem(cpu.regs[k - 0x10] + next_pc(cpu)) }
      0x18 { let r = value_mem(cpu.sp); cpu.sp += 1u16; r }
      0x19 { value_mem(cpu.sp) }
      0x1a { cpu.sp -= 1u16; value_mem(cpu.sp) }
      0x1b { value_sp }
      0x1c { value_pc }
      0x1d { value_o  }
      0x1e { value_mem(next_pc(cpu)) }
      0x1f { value_literal(next_pc(cpu)) }
      0x20 to 0x3f { value_literal(key - 0x20u16) }
      _ { error("get_val: invalid value"); fail; }
    }
}

fn get_value(cpu: cpu_state, v: value) -> u16 {
    alt v {
      value_reg(t)     { cpu.regs[t] }
      value_mem(t)     { cpu.mem[t] }
      value_sp         { cpu.sp }
      value_pc         { cpu.pc }
      value_o          { cpu.o  }
      value_literal(t) { t }
    }
}
                       
fn set_value(cpu: cpu_state, targ: value, v: u16) {
    alt targ {
      value_reg(t) { cpu.regs[t] = v; }
      value_mem(t) { cpu.mem[t] = v; }
      value_sp     { cpu.sp = v; }
      value_pc     { cpu.pc = v; }
      value_o      { cpu.o = v; }
      value_literal(t) { error("set_val: attempt to set a literal"); }
    }
}

fn step(cpu: cpu_state) {

    let word = next_pc(cpu);
    let op   =       basic_op((word & 0b0000000000001111u16) as uint);
    let av   = new_value(cpu, (word & 0b0000001111110000u16) >> 4u16);
    let bv   = new_value(cpu, (word & 0b1111110000000000u16) >> 10u16);

    let a = get_value(cpu, av) as uint;
    let b = get_value(cpu, bv) as uint;

    // Non-basic instructions
    if op == NBI {        
        alt special_op(((word & 0b0000001111110000u16) >> 4u16) as uint) {
          // use the word 0x0000 this as a 'stop' command
          EXT { cpu.stop = true }
          JSR {
            cpu.sp -= 1u16;
            cpu.mem[cpu.sp] = next_pc(cpu);
            cpu.pc = b as u16;
            cpu.cycles += 2u
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
          _ { 0u }
        };

        // special logic
        alt op {
          ADD | SUB | MUL | DIV | SHL | SHR  {
            cpu.o = (res >> 16) as u16;
            set_value(cpu, av, (res & 0xFFFFu) as u16);
          }
          SET | MOD |  AND | BOR | XOR {
            set_value(cpu, av, (res & 0xFFFFu) as u16);
          }
          IFE | IFN | IFG | IFB {
            cpu.pc += 1u16 - (res as u16); // if res is true (1) we don't skip
          }
          _ { }
        }
    }
}

fn dump_header() {
    io::println(" A     B     C     X     Y     Z     I     J     PC    SP    O   cycles");
    io::println("-----------------------------------------------------------------------");
}

fn dump_state(cpu: cpu_state) {
    io::println(#fmt(
        "%04x  %04x  %04x  %04x  %04x  %04x  %04x  %04x  %04x  %04x  %04x   %u",
        cpu.regs[0] as uint, cpu.regs[1] as uint, cpu.regs[2] as uint,
        cpu.regs[3] as uint, cpu.regs[4] as uint, cpu.regs[5] as uint,
        cpu.regs[6] as uint, cpu.regs[7] as uint, cpu.pc as uint,
        cpu.sp as uint, cpu.o as uint, cpu.cycles
    ));
}

// A rom file is an ascii text file containing raw cpu instructions.
// All text in the file is ignored except for hexadecimal characters
// found between a single pair of {{ and }}.
// example rom file:
//    This program sets A to 0x30
//    {{ 7c01 0030 }}
//    more text that is ignored
fn load_rom(cpu: cpu_state, filename: str) {
    let input = result::get(io::read_whole_file_str(filename));

    let start = str::find_str(input, "{{");
    let end   = str::find_str(input, "}}");

    if option::is_none(start) || option::is_none(end)  {
        error("invalid rom file");
        ret;
    }

    let s = option::get(start);
    let e = option::get(end);

    if s > e {
        error("invalid rom file");
        ret;
    }

    let data = str::slice(input, s + 2u, e);
    let mut chars : [u8] = [];

    for str::each(data) {|c|
        alt c {
          48u8 to 57u8 { vec::push(chars, c - 48u8); }
          65u8 to 70u8 { vec::push(chars, c - 65u8 + 10u8); }
          97u8 to 102u8 { vec::push(chars, c - 97u8 + 10u8); }
          _ { }
        }
    }

    if vec::len(chars) % 4u != 0u {
        error("invalid rom file, invalid number of bytes");
        ret;
    }

    let max = vec::len(chars) / 4u;
    let mut i = 0u;
    while i < max {
        let k = (i * 4u) as int;
        cpu.mem[i] = (
            (chars[k]   as uint << 12u) |
            (chars[k+1] as uint << 8u)  |
            (chars[k+2] as uint << 4u)  |
            (chars[k+3] as uint )
        ) as u16;
        i += 1u;
    }
}

fn main() {
    
    dump_header();
    let cpu = new_cpu_state();
    load_rom(cpu, "example.rom");

    while !cpu.stop {
        dump_state(cpu);
        step(cpu);
    }
    
}
