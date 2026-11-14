pub enum V {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<V>),
    Obj(Vec<(String, V)>),
}

static NULL: V = V::Null;

impl V {
    pub fn get(&self, k: &str) -> &V {
        match self {
            V::Obj(m) => m.iter().find(|(a, _)| a == k).map_or(&NULL, |(_, v)| v),
            _ => &NULL,
        }
    }

    pub fn arr(&self) -> &[V] {
        match self {
            V::Arr(a) => a,
            _ => &[],
        }
    }

    pub fn str(&self) -> &str {
        match self {
            V::Str(s) => s,
            _ => "",
        }
    }

    pub fn num(&self) -> f64 {
        match self {
            V::Num(n) => *n,
            V::Bool(b) => *b as u8 as f64,
            _ => 0.0,
        }
    }
}

pub fn parse(s: &str) -> Option<V> {
    P { b: s.as_bytes(), i: 0 }.val()
}

struct P<'a> {
    b: &'a [u8],
    i: usize,
}

impl P<'_> {
    fn ws(&mut self) {
        while matches!(self.b.get(self.i), Some(b' ' | b'\t' | b'\r' | b'\n')) {
            self.i += 1;
        }
    }

    fn eat(&mut self, c: u8) -> bool {
        self.ws();
        if self.b.get(self.i) == Some(&c) {
            self.i += 1;
            true
        } else {
            false
        }
    }

    fn lit(&mut self, s: &str, v: V) -> Option<V> {
        if self.b[self.i..].starts_with(s.as_bytes()) {
            self.i += s.len();
            Some(v)
        } else {
            None
        }
    }

    fn val(&mut self) -> Option<V> {
        self.ws();
        match *self.b.get(self.i)? {
            b'{' => self.obj(),
            b'[' => self.arr(),
            b'"' => self.str().map(V::Str),
            b't' => self.lit("true", V::Bool(true)),
            b'f' => self.lit("false", V::Bool(false)),
            b'n' => self.lit("null", V::Null),
            _ => self.num(),
        }
    }

    fn obj(&mut self) -> Option<V> {
        self.i += 1;
        let mut m = Vec::new();
        if self.eat(b'}') {
            return Some(V::Obj(m));
        }
        loop {
            self.ws();
            let k = self.str()?;
            if !self.eat(b':') {
                return None;
            }
            m.push((k, self.val()?));
            if self.eat(b',') {
                continue;
            }
            return self.eat(b'}').then_some(V::Obj(m));
        }
    }

    fn arr(&mut self) -> Option<V> {
        self.i += 1;
        let mut a = Vec::new();
        if self.eat(b']') {
            return Some(V::Arr(a));
        }
        loop {
            a.push(self.val()?);
            if self.eat(b',') {
                continue;
            }
            return self.eat(b']').then_some(V::Arr(a));
        }
    }

    fn hex(&mut self) -> Option<u32> {
        let h = std::str::from_utf8(self.b.get(self.i..self.i + 4)?).ok()?;
        self.i += 4;
        u32::from_str_radix(h, 16).ok()
    }

    fn str(&mut self) -> Option<String> {
        if self.b.get(self.i) != Some(&b'"') {
            return None;
        }
        self.i += 1;
        let mut o = Vec::new();
        loop {
            let c = *self.b.get(self.i)?;
            self.i += 1;
            match c {
                b'"' => return Some(String::from_utf8_lossy(&o).into_owned()),
                b'\\' => {
                    let e = *self.b.get(self.i)?;
                    self.i += 1;
                    let ch = match e {
                        b'n' => '\n',
                        b't' => '\t',
                        b'r' => '\r',
                        b'b' => '\u{8}',
                        b'f' => '\u{c}',
                        b'u' => {
                            let mut u = self.hex()?;
                            if (0xD800..0xDC00).contains(&u) && self.b[self.i..].starts_with(b"\\u") {
                                self.i += 2;
                                let l = self.hex()?;
                                u = 0x10000 + ((u - 0xD800) << 10) + (l.wrapping_sub(0xDC00) & 0x3FF);
                            }
                            char::from_u32(u).unwrap_or('\u{FFFD}')
                        }
                        c => c as char,
                    };
                    let mut buf = [0u8; 4];
                    o.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                }
                c => o.push(c),
            }
        }
    }

    fn num(&mut self) -> Option<V> {
        let s = self.i;
        while matches!(self.b.get(self.i), Some(b'0'..=b'9' | b'-' | b'+' | b'.' | b'e' | b'E')) {
            self.i += 1;
        }
        std::str::from_utf8(&self.b[s..self.i]).ok()?.parse().ok().map(V::Num)
    }
}
