
use std;

import io::reader_util;
import result::{result, err, ok, extensions};
import std::map::hashmap;

// An argument to an instruction
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

fn value_size(v: value) -> u16 {
    alt v {
      value_data1(_)    { 0u16 }
      value_data2(_, _) { 1u16 }
      value_label(_)    { 1u16 }
    }
}

type instruction = {
    mut o : u16,   // opcode
    mut a : value, // argument 1 (a)
    mut b : value  // argument 2 (b)
};

fn new_instruction(o: u16, a: value, b: value) -> instruction {
    {
        mut o: o,
        mut a: a,
        mut b: b
    }
}

fn instruction_size(i: instruction) -> u16 {
    value_size(i.a) + value_size(i.b) + 1u16
}

fn instruction_bytes(i: instruction) -> [u16] {
    let mut first = i.o;
    let mut bytes = [];
    alt i.a {
      value_data1(a)    { first |= a << 4u16; }
      value_data2(a, b) { first |= a << 4u16; bytes += [b]; }
      _ { }
    }
    alt i.b {
      value_data1(a)    { first |= a << 10u16; }
      value_data2(a, b) { first |= a << 10u16; bytes += [b]; }
      _ { }
    }
    [first] + bytes
}

fn print_instruction(i: instruction) {
    io::println(#fmt("%x %s %s", i.o as uint, value_str(i.a), value_str(i.b)));
}

fn is_num(p: str) -> bool {
    import iter::*;
    let digits = iter::to_vec(uint::range(0u, 9u, _));
    let digits = digits.map {|d| #fmt("%u", d) };
    digits.any {|d| p.trim().starts_with(d) }
}

// parse an interger literal, hex or decimal.
fn parse_num(p:str) -> result<u16, str> {
    let num = if p.starts_with("0x") {
        let buf = str::bytes(str::replace(p, "0x", ""));
        uint::parse_buf(buf, 16u)
    } else {
        let buf = str::bytes(p);
        uint::parse_buf(buf, 10u)
    };
    if num.is_none() {
        ret err("Invalid integer literal");
    }
    if num.get() > 0xFFFFu {
        ret err("Integer literal too large (max 0xFFFF)");
    }
    ret result::ok(num.get() as u16);
}

// return the ID associated with a register
fn parse_reg(p:u8) -> result<u16, str> {
    ok(alt p as char {
      'A' { 0 } 'B' { 1 } 'C' { 2 } 'X' { 3 }
      'Y' { 4 } 'Z' { 5 } 'I' { 6 } 'J' { 7 }
      _   { ret err("Invalid register name " + str::from_char(p as char)); }
    } as u16)
}

fn remove_brackets(v: str) -> str {
    str::replace(str::replace(v, "[", ""), "]", "")
}

fn valid_label(v: str) -> bool {
    if (v.len() == 0u) { ret false; }
    !char::is_digit(str::char_at(v, 0u)) && !v.any { |c|
        !(char::is_alphanumeric(c) || c == '_' || c == '-' || c == '$')
    }
}

// Parse an instruction argument
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

        let (reg, word) = if !is_num(left) {
            (parse_reg(left[0]), parse_num(right)) // reg + num
        } else {
            (parse_reg(right[0]), parse_num(left)) // num + reg
        };

        if reg.is_failure() { ret err(reg.get_err()); }
        if word.is_failure() { ret err(word.get_err()); }
        ret ok(value_data2(reg.get() + 0x10u16, word.get()));
    }

    // [next word]
    if str::find_char(part, '[').is_some() {
        ret result::chain(parse_num(remove_brackets(part))) { |t|
            ok(value_data2(0x1Eu16, t))
        }
    }

    // next word literal or inline literal
    if is_num(part) {
        ret result::chain(parse_num(part)) { |t|
            if t <= 0x1Fu16 {
                ok(value_data1(0x20u16 + t))
            } else {
                ok(value_data2(0x1Fu16, t))
            }
        }
    // label
    } else {
        if !valid_label(part) {
            ret err("Expected valid label (letters, numbers, _, -, or $)");
        }
        ret ok(value_label(part));
    }
}

fn get_op(cmd:str) -> result<u16,str> {
    ok(alt cmd {
      "SET" { 1  }
      "ADD" { 2  }
      "SUB" { 3  }
      "MUL" { 4  }
      "DIV" { 5  }
      "MOD" { 6  }
      "SHL" { 7  }
      "SHR" { 8  }
      "AND" { 9  }
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

    io::println("Line: " + line);
    let mut parts = str::words(line);
    let cmd = vec::shift(parts);
    let args = str::concat(parts).split_char(',');

    if args.any ({|s| s.is_empty()}) { ret err("Empty argument"); }

    if cmd == "JSR" {
        if args.len() != 1u {
            ret err("Wrong number of arguments for JSR");
        }
        ret result::chain(make_val(args[0])) { |t|
            ok(new_instruction(0u16, value_data1(1u16), t))
        }
    } else {

        if args.len() != 2u {
            ret err("Wrong number of arguments (expected 2)");
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

fn perr(line: uint, msg: str) {
    io::println(#fmt("Compile error on line %u: %s", line, msg));
}

fn compile_file(filename: str)
{
    let r = io::file_reader(filename);
    if r.is_failure() {
        io::println("Could not open specified file");
        ret
    }

    let mut instrs : [mut instruction] = [mut];
    let mut line_no = 0u;
    let mut word_no = 0u16;
    let labels = std::map::hashmap::<str, u16>(str::hash, str::eq);
    let rdr = r.get();

    while !rdr.eof() {
        let mut line = str::trim(rdr.read_line());
        line_no += 1u;

        let comment = str::find_char(line, ';');
        if !comment.is_none() {
            line = str::trim(str::substr(line, 0u, comment.get()));
        }
        if line.is_empty() { cont; }

        let mut label = str::words(line)[0];
        if str::pop_char(label) == ':' {
            if !valid_label(label) {
                perr(line_no, "invalid label definition");
                ret;
            }
            labels.insert(label, word_no);
            line = line.split_char(':')[1].trim();
        }

        if line.is_empty() { cont; }

        let res = compile_line(line);

        if res.is_failure() {
            perr(line_no, res.get_err());
            ret;
        }


        word_no += instruction_size(res.get());

        vec::push(instrs, res.get());
    }

    io::println("Done parsing");

    for instrs.each {|i|
        let k = [i.a, i.b];
        let f = vec::map(k) {|v|
            alt v {
              value_label(a) {
                if !labels.contains_key(a) {
                    perr(line_no, "invalid label reference");
                }
                value_data2(0x1fu16, labels.get(a))
              }
              _ { v }
            }
        };
        i.a = f[0]; i.b = f[1];
    }

    let mut bytes : [u16] = [];
    for instrs.each {|i|
        bytes += instruction_bytes(i);
    };

    io::println("rust-dcpu-16 generated ROM");
    io::println("{{");

    for bytes.each { |num|
        let mut hex = uint::to_str(num as uint, 16u);
        iter::repeat(4u - hex.len()) {|| hex = "0" + hex};
        io::println("  " + hex);
    }

    io::println("}}");
}

fn main(args: [str]) {
    if (args.len() != 2u) {
        io::println("Usage:");
        io::println("  ./asm [asm file]");
        ret;
    }

    compile_file(args[1]);
}
