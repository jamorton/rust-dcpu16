
import io::reader_util;
import result::result;
import result::err;
import result::extensions;

// Parse a hexadecimal number
fn parse_num(p:str) -> result<u16, str> {
    if str::find_str(p, "0x").is_none() {
        ret err("expecting 0x");
    }
    let mut buf : [u8] = [];
    for str::replace(p, "0x", "").each {|t| vec::push(buf, t as u8); };
    let num = uint::parse_buf(buf, 16u);
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
    result::ok(alt p as char {
      'A' { 0 } 'B' { 1 } 'C' { 2 } 'X' { 3 }
      'Y' { 4 } 'Z' { 5 } 'I' { 6 } 'J' { 7 }
      _   { ret err("invalid register name"); }
    } as u16)
}

fn remove_brackets(v: str) -> str {
    str::replace(str::replace(v, "[", ""), "]", "")
}

// Parse an argument to an opcode
fn make_val(part:str) -> result<[u16], str> {

    alt part {
      "POP"  { ret result::ok([0x18u16]); } "PEEK" { ret result::ok([0x19u16]); }
      "PUSH" { ret result::ok([0x1Au16]); } "SP"   { ret result::ok([0x1Bu16]); }
      "PC"   { ret result::ok([0x1Cu16]); } "O"    { ret result::ok([0x1Du16]); }
      _      { }
    }

    // 'A'
    if part.len() == 1u {
        ret result::chain(parse_reg(part[0])) { |t|  result::ok([t]) };
    }

    // '[A]'
    if part.len() == 3u && part[0] == ('[' as u8) && part[2] == (']' as u8) {
        ret result::chain(parse_reg(part[1])) { |t| result::ok([t + 0x08u16]) };
    }

    // '[0x1000 + a]'
    if !str::find_char(part, '+').is_none() {
        let v = remove_brackets(part).split_char('+');

        let (reg, word) = if str::find_str(v[0], "0x").is_none() {
            if (str::len(v[0]) != 1u) { ret err("expected register"); }
            (parse_reg(v[0][0]), parse_num(v[1]))
        } else {
            if (str::len(v[1]) != 1u) { ret err("expected register"); }
            (parse_reg(v[1][0]), parse_num(v[0]))
        };
        
        if reg.is_failure() { ret err(reg.get_err()); }
        if word.is_failure() { ret err(word.get_err()); }
        ret result::ok([reg.get() + 0x10u16, word.get()]);
    }

    // '[0x1000]'
    if !str::find_char(part, '[').is_none() {
        ret result::chain(parse_num(remove_brackets(part))) { |t|
            result::ok([0x1Eu16, t])
        }
    }

    // '0x1000'
    ret result::chain(parse_num(part)) { |t|
        result::ok([0x1Fu16, t])
    }

}

fn get_op(cmd:str) -> result<u16,str> {
    result::ok(alt cmd {
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

fn compile_line(l:str) -> result<[u16],str> {
    let mut line = str::trim(l);
    let comment = str::find_char(line, ';');
    if !comment.is_none() {
        line = str::trim(str::substr(line, 0u, comment.get()));
    }
    if line.is_empty() { ret result::ok([]); }

    let mut parts = str::words(line);
    let cmd = vec::shift(parts);
    let args = str::concat(parts).split_char(',');

    if cmd == "JSR" {
        // TODO 
    } else {

        let mut word : u16 = 0u16;
        let mut final : [u16] = [];
        let k = get_op(cmd);
        if k.is_failure() { ret err(k.get_err()); }
        word |= k.get();
        
        if args.len() != 2u {
            ret err("wrong number of arguments");
        }
        
        let a = make_val(args[0]);
        let b = make_val(args[1]);
        if a.is_failure() { ret err(a.get_err()); }
        if b.is_failure() { ret err(b.get_err()); }

        let av = a.get();
        let bv = b.get();

        word |= (av[0] & 0b111111u16) << 4u16;
        word |= (bv[0] & 0b111111u16) << 10u16;
        vec::push(final, word);

        if av.len() == 2u {
            vec::push(final, av[1]);
        }
        if bv.len() == 2u {
            vec::push(final, bv[1]);
        }

        ret result::ok(final);
    }
    
    ret err("not implemented");
}

fn compile_file(filename: str)
{
    let r = io::file_reader(filename);
    if r.is_failure() {
        io::println("could not open file");
    }

    let mut out : [u16] = [];
    let mut n = 0u;
    let rdr = r.get();
    
    while !rdr.eof() {
        n += 1u;
        let line = rdr.read_line();
        let res = compile_line(line);

        if res.is_failure() {
            io::println(#fmt("Compile error: %s on line %u", res.get_err(), n));
            ret;
        }
        
        for res.get().each { |t| vec::push(out, t); }
    }

    io::println("rust-dcpu-16 generated ROM");
    io::println("{{");

    for out.each { |num|
        let mut hex = uint::to_str(num as uint, 16u);
        iter::repeat(4u - hex.len()) {|| hex = "0" + hex};
        io::println("  " + hex);
    }
    
    io::println("}}");
}

fn main(args: [str]) {
    compile_file(args[1]);

}
