
import io::reader_util;
import result::{result, err, ok, extensions};

enum value {
    value_data1(u16),      // basic value data
    value_data2(u16, u16), // for the 'next word' constructs
    value_label(str)       // labels to be patched later
}

fn value_str(v: value) -> str {
    alt v {
      value_data1(a)    { uint::to_str(a as uint, 16u) }
      value_data2(a, b) {
        uint::to_str(a as uint, 16u) + " " + uint::to_str(b as uint, 16u)
      }
      value_label(a)    { a }
    }
}

type instruction = {
    mut o : u16,
    mut a : value,
    mut b : value
};

fn new_instruction(o: u16, a: value, b: value) -> instruction {
    {
        mut o: o,
        mut a: a,
        mut b: b
    }
}

fn print_instruction(i: instruction) {
    io::println(#fmt("%x %s %s", i.o as uint, value_str(i.a), value_str(i.b)));
}

fn to_bytes(p:str) -> [u8] {
    let mut buf : [u8] = [];
    for p.each {|t| vec::push(buf, t)};
    buf
}

fn is_num(p: str) -> bool {
    import iter::*;
    let digits = iter::to_vec(uint::range(0u, 9u, _));
    let digits = digits.map {|d| #fmt("%u", d) };
    digits.any {|d| p.trim().starts_with(d) }
}

fn parse_num(p:str) -> result<u16, str> {
    let num = if p.starts_with("0x") {
        let mut buf : [u8] = [];
        for str::replace(p, "0x", "").each {|t| vec::push(buf, t as u8); };
        uint::parse_buf(buf, 16u)
    } else {
        let buf = str::bytes(p);
        uint::parse_buf(buf, 10u)
    };
    if num.is_none() {
        ret err("invalid number");
    }
    if num.get() > 0xFFFFu {
        ret err("constant too large");
    }
    ret result::ok(num.get() as u16);
}

// return the ID associated with a register
fn parse_reg(p:u8) -> result<u16, str> {
    ok(alt p as char {
      'A' { 0 } 'B' { 1 } 'C' { 2 } 'X' { 3 }
      'Y' { 4 } 'Z' { 5 } 'I' { 6 } 'J' { 7 }
      _   { ret err("invalid register name"); }
    } as u16)
}

fn remove_brackets(v: str) -> str {
    str::replace(str::replace(v, "[", ""), "]", "")
}

// Parse an argument to an opcode
fn make_val(part:str) -> result<value, str> {

    // simple values
    alt part {
      "POP"  { ret ok(value_data1(0x18u16)); }
      "PEEK" { ret ok(value_data1(0x19u16)); }
      "PUSH" { ret ok(value_data1(0x1Au16)); }
      "SP"   { ret ok(value_data1(0x1Bu16)); }
      "PC"   { ret ok(value_data1(0x1Cu16)); }
      "O"    { ret ok(value_data1(0x1Du16)); }
      _      { }
    }

    // register
    let reg_res = result::chain(parse_reg(part[0])) { |t| ok(value_data1(t)) };
    if result::is_success(reg_res) {
        ret reg_res;
    } else {
        #debug("didn't parse a reg: %?", reg_res);
    }

    // [register]
    if part.len() == 3u && part[0] == ('[' as u8) && part[2] == (']' as u8) {
        ret result::chain(parse_reg(part[1])) { |t|
            ok(value_data1(t + 0x08u16))
        };
    }

    // [next word + register]
    if !str::find_char(part, '+').is_none() {
        let v = remove_brackets(part).split_char('+');
        let left =  v[0];
        let right = v[1];

        if left.len() != 1u || right.len() != 1u {
            ret err("expected register");
        }

        let (reg, word) = if !is_num(left) {
            // reg + num
            (parse_reg(left[0]), parse_num(right))
        } else {
            // num + reg
            (parse_reg(right[0]), parse_num(left))
        };

        if reg.is_failure() { ret err(reg.get_err()); }
        if word.is_failure() { ret err(word.get_err()); }
        ret ok(value_data2(reg.get() + 0x10u16, word.get()));
    }

    // [next word]
    if !str::find_char(part, '[').is_none() {
        ret result::chain(parse_num(remove_brackets(part))) { |t|
            ok(value_data2(0x1Eu16, t))
        }
    }

    // next word literal or inline literal
    if char::is_digit(str::char_at(part, 0u)) {
        ret result::chain(parse_num(part)) { |t|
            if t <= 0x1Fu16 {
                ok(value_data1(0x20u16 + t))
            } else {
                ok(value_data2(0x1Fu16, t))
            }
        }
    // label
    } else {
        ret ok(value_label(part))
    }
}

fn get_op(cmd:str) -> result<u16,str> {
    ok(alt cmd {
      "SET" { 1 }
      "ADD" { 2 }
      "SUB" { 3 }
      "MUL" { 4 }
      "DIV" { 5 }
      "MOD" { 6 }
      "SHL" { 7 }
      "SHR" { 8 }
      "AND" { 9 }
      "BOR" { 10 }
      "XOR" { 11 }
      "IFE" { 12 }
      "IFN" { 13 }
      "IFG" { 14 }
      "IFB" { 15 }
      _     { ret err("invalid opcode"); }
    } as u16)
}

fn compile_line(line:str) -> result<instruction,str> {

    let mut parts = str::words(line);
    let cmd = vec::shift(parts);
    let args = str::concat(parts).split_char(',');

    if cmd == "JSR" {
        if args.len() != 1u {
            ret err("wrong number of arguments for JSR");
        }
        ret result::chain(make_val(args[0])) { |t|
            ok(new_instruction(0u16, value_data1(1u16), t))
        }
    } else {

        if args.len() != 2u {
            ret err("wrong number of arguments");
        }

        let op = get_op(cmd);
        let a  = make_val(args[0]);
        let b  = make_val(args[1]);
        if a.is_failure()  { ret err(a.get_err()); }
        if b.is_failure()  { ret err(b.get_err()); }
        if op.is_failure() { ret err(op.get_err()); }

        ret ok(new_instruction(op.get(), a.get(), b.get()));
    }
}

fn compile_file(filename: str)
{
    let r = io::file_reader(filename);
    if r.is_failure() {
        io::println("could not open file");
    }

    let mut instrs : [instruction] = [];
    let mut n = 0u;
    let rdr = r.get();

    while !rdr.eof() {
        let mut line = str::trim(rdr.read_line());

        let comment = str::find_char(line, ';');
        if !comment.is_none() {
            line = str::trim(str::substr(line, 0u, comment.get()));
        }
        if line.is_empty() { cont; }

        n += 1u;
        let res = compile_line(line);

        if res.is_failure() {
            io::println(#fmt("Compile error: %s on line %u", res.get_err(), n));
            ret;
        }

        vec::push(instrs, res.get());
    }

    for instrs.each {|i| print_instruction(i)};

    /*
    io::println("rust-dcpu-16 generated ROM");
    io::println("{{");

    for out.each { |num|
        let mut hex = uint::to_str(num as uint, 16u);
        iter::repeat(4u - hex.len()) {|| hex = "0" + hex};
        io::println("  " + hex);
    }

    io::println("}}");
    */
}

fn main(args: [str]) {
    compile_file(args[1]);

}
