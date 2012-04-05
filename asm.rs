
import io::reader_util;
import result::result;

fn parse_num(p:str) -> result<u16, str> {
    if str::find_str(p, "0x").is_none() {
        ret result::err("expecting 0x");
    }
    let mut buf : [u8] = [];
    for str::replace(p, "0x", "").each {|t| vec::push(buf, t as u8); };
    let num = uint::parse_buf(buf, 16u);
    if num.is_none() {
        ret result::err("invalid number");
    }
    if num.get() > 0xFFFFu {
        ret result::err("constant too large");
    }
    ret result::ok(num.get() as u16);
}

fn parse_reg(p:u8) -> result<u16, str> {
    alt p as char {
      'A' { result::ok(0u16) } 'B' { result::ok(1u16) } 'C' { result::ok(2u16) } 'X' { result::ok(3u16) }
      'Y' { result::ok(4u16) } 'Z' { result::ok(5u16) } 'I' { result::ok(6u16) } 'J' { result::ok(7u16) }
      _   { result::err("invalid register name") }
    }
}

fn make_val(part:str) -> result<[u16], str> {

    alt part {
      "POP"  { ret result::ok([0x18u16]); } "PEEK" { ret result::ok([0x19u16]); }
      "PUSH" { ret result::ok([0x1Au16]); } "SP"   { ret result::ok([0x1Bu16]); }
      "PC"   { ret result::ok([0x1Cu16]); } "O"    { ret result::ok([0x1Du16]); }
      _ { }
    }

    if str::len(part) == 1u {
        ret result::chain(parse_reg(part[0])) { |t|  result::ok([t]) };
    }

    if str::len(part) == 3u && part[0] == ('[' as u8) && part[2] == (']' as u8) {
        ret result::chain(parse_reg(part[1])) { |t| result::ok([t + 0x08u16]) };
    }

    if !str::find_char(part, '+').is_none() {
        let v = str::replace(str::replace(part, "[", ""), "]", "").split_char('+');
        let (reg, word) = if str::find_str(v[0], "0x").is_none() {
            if (str::len(v[0]) != 1u) {
                ret result::err("expected register");
            }
            (parse_reg(v[0][0]), parse_num(v[1]))
        } else {
            if (str::len(v[1]) != 1u) {
                ret result::err("expected register");
            }
            (parse_reg(v[1][0]), parse_num(v[0]))
        };
        if result::is_failure(reg) { ret result::err(result::get_err(reg)); }
        if result::is_failure(word) { ret result::err(result::get_err(word)); }
        ret result::ok([result::get(reg) + 0x10u16, result::get(word)]);
    }

    if !str::find_char(part, '[').is_none() {
        ret result::chain(parse_num(str::replace(str::replace(part, "[", ""), "]", ""))) { |t|
            result::ok([0x1Eu16, t])
        }
    }

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
      _     { ret result::err("invalid opcode"); }
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
        if result::is_failure(k) { ret result::err(result::get_err(k)); }
        word |= result::get(k);
        
        if args.len() != 2u {
            ret result::err("wrong number of arguments");
        }
        let a = make_val(args[0]);
        let b = make_val(args[1]);
        if result::is_failure(a) { ret result::err(result::get_err(a)); }
        if result::is_failure(b) { ret result::err(result::get_err(b)); }

        let av = result::get(a);
        let bv = result::get(b);

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
    
    ret result::err("not implemented");
}

fn compile_file(filename: str)
{
    let r = io::file_reader(filename);
    if result::is_failure(r) {
        io::println("could not open file");
    }

    let mut out : [u16] = [];
    let mut n = 0u;
    let rdr = result::get(r);
    while !rdr.eof() {
        n += 1u;
        let line = rdr.read_line();
        let res = compile_line(line);
        
        if result::is_failure(res) { io::println(#fmt("Compile error: %s on line %u", result::get_err(res), n)); ret; }
        for result::get(res).each { |t| vec::push(out, t); }
    }

    io::println("{{");

    for out.each { |num|
        let mut o = uint::to_str(num as uint, 16u);
        let mut len = str::len(o);
        while len < 4u {
            o = "0" + o;
            len += 1u
        }
        io::println(o);
    }
    io::println("}}");
}

fn main(args: [str]) {
    compile_file(args[1]);

}
