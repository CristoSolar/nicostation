//! Expresiones: `Player.x + 10`, `"Puntos: " + score`, `random(0, 100)`, `count(Coin)`.
//! Operadores + - * / % con asociatividad izquierda (a-b-c = (a-b)-c), comparaciones
//! (== != < > <= >=; también `=`) y lógica (and, or, not). Verdadero = 1, falso = 0.

use crate::model::Value;

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Num(f64),
    Str(String),
    /// Variable (local, de escena o global) o nombre suelto.
    Var(String),
    /// `Objeto.propiedad`
    Prop(String, String),
    Call(String, Vec<Expr>),
    Neg(Box<Expr>),
    Bin(u8, Box<Expr>, Box<Expr>),
    /// Comparación: "==", "!=", "<", ">", "<=", ">=".
    Cmp(&'static str, Box<Expr>, Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Not(Box<Expr>),
}

/// Lo que la expresión necesita del juego.
pub trait Ctx {
    fn var(&self, name: &str) -> Value;
    fn prop(&self, obj: &str, prop: &str) -> Value;
    /// Funciones del juego (count, mousex...). Recibe los argumentos sin evaluar.
    fn call(&self, name: &str, args: &[Expr]) -> Option<Value>;
    fn random(&self) -> f64;
}

pub fn parse(src: &str) -> Result<Expr, String> {
    let mut p = Parser { s: src.as_bytes(), i: 0 };
    let e = p.or()?;
    p.ws();
    if p.i < p.s.len() {
        return Err(format!("sobra texto en la posición {}: «{}»", p.i, &src[p.i..]));
    }
    Ok(e)
}

pub fn eval(e: &Expr, c: &dyn Ctx) -> Value {
    match e {
        Expr::Num(n) => Value::Num(*n),
        Expr::Str(s) => Value::Str(s.clone()),
        Expr::Var(v) => c.var(v),
        Expr::Prop(o, p) => c.prop(o, p),
        Expr::Neg(a) => Value::Num(-eval(a, c).num()),
        Expr::Not(a) => bool_v(!eval(a, c).truthy()),
        Expr::And(a, b) => bool_v(eval(a, c).truthy() && eval(b, c).truthy()),
        Expr::Or(a, b) => bool_v(eval(a, c).truthy() || eval(b, c).truthy()),
        Expr::Cmp(op, a, b) => {
            let (a, b) = (eval(a, c), eval(b, c));
            let ord = match (&a, &b) {
                (Value::Str(x), Value::Str(y)) => x.cmp(y),
                _ => a.num().partial_cmp(&b.num()).unwrap_or(std::cmp::Ordering::Equal),
            };
            use std::cmp::Ordering::*;
            bool_v(match *op {
                "==" => ord == Equal,
                "!=" => ord != Equal,
                "<" => ord == Less,
                ">" => ord == Greater,
                "<=" => ord != Greater,
                _ => ord != Less,
            })
        }
        Expr::Bin(op, a, b) => {
            let (a, b) = (eval(a, c), eval(b, c));
            if *op == b'+' && (matches!(a, Value::Str(_)) || matches!(b, Value::Str(_))) {
                return Value::Str(format!("{}{}", a.text(), b.text()));
            }
            let (a, b) = (a.num(), b.num());
            Value::Num(match op {
                b'+' => a + b,
                b'-' => a - b,
                b'*' => a * b,
                b'/' => if b == 0. { 0. } else { a / b },
                _ => if b == 0. { 0. } else { a.rem_euclid(b) },
            })
        }
        Expr::Call(f, args) => {
            if let Some(v) = c.call(f, args) {
                return v;
            }
            let n = |i: usize| args.get(i).map(|a| eval(a, c).num()).unwrap_or(0.);
            let s = |i: usize| args.get(i).map(|a| eval(a, c).text()).unwrap_or_default();
            match f.as_str() {
                "random" => Value::Num(n(0) + c.random() * (n(1) - n(0))),
                "randint" => Value::Num((n(0) + c.random() * (n(1) - n(0) + 1.)).floor().min(n(1))),
                "abs" => Value::Num(n(0).abs()),
                "min" => Value::Num(n(0).min(n(1))),
                "max" => Value::Num(n(0).max(n(1))),
                "clamp" => Value::Num(n(0).clamp(n(1), n(2).max(n(1)))),
                "floor" => Value::Num(n(0).floor()),
                "ceil" => Value::Num(n(0).ceil()),
                "round" => Value::Num(n(0).round()),
                "sqrt" => Value::Num(n(0).max(0.).sqrt()),
                "pow" => Value::Num(n(0).powf(n(1))),
                "sin" => Value::Num(n(0).to_radians().sin()),
                "cos" => Value::Num(n(0).to_radians().cos()),
                "lerp" => Value::Num(n(0) + (n(1) - n(0)) * n(2)),
                "if" => args.get(if eval(&args[0], c).truthy() { 1 } else { 2 }).map(|a| eval(a, c)).unwrap_or(Value::Num(0.)),
                "sign" => Value::Num(if n(0) > 0. { 1. } else if n(0) < 0. { -1. } else { 0. }),
                "atan2" => Value::Num(n(0).atan2(n(1)).to_degrees()),
                "lengthdir_x" => Value::Num(n(0) * n(1).to_radians().cos()),
                "lengthdir_y" => Value::Num(n(0) * n(1).to_radians().sin()),
                "tan" => Value::Num(n(0).to_radians().tan()),
                "substr" => Value::Str(s(0).chars().skip(n(1).max(0.) as usize).take(if args.len() > 2 { n(2).max(0.) as usize } else { usize::MAX }).collect()),
                "contains" => bool_v(s(0).contains(&s(1))),
                "replace" => Value::Str(s(0).replace(&s(1), &s(2))),
                "fixed" => Value::Str(format!("{:.*}", n(1).clamp(0., 10.) as usize, n(0))),
                "pad" => Value::Str(format!("{:0>w$}", s(0), w = n(1).clamp(0., 32.) as usize)),
                "angle" => Value::Num((n(3) - n(1)).atan2(n(2) - n(0)).to_degrees()),
                "distance" => Value::Num((n(2) - n(0)).hypot(n(3) - n(1))),
                "str" => Value::Str(s(0)),
                "num" => Value::Num(Value::Str(s(0)).num()),
                "len" => Value::Num(s(0).chars().count() as f64),
                "upper" => Value::Str(s(0).to_uppercase()),
                "lower" => Value::Str(s(0).to_lowercase()),
                _ => Value::Num(0.),
            }
        }
    }
}

fn bool_v(b: bool) -> Value {
    Value::Num(b as i32 as f64)
}

struct Parser<'a> {
    s: &'a [u8],
    i: usize,
}

impl Parser<'_> {
    fn ws(&mut self) {
        while self.s.get(self.i).is_some_and(|c| c.is_ascii_whitespace()) {
            self.i += 1;
        }
    }
    fn eat(&mut self, c: u8) -> bool {
        self.ws();
        let ok = self.s.get(self.i) == Some(&c);
        self.i += ok as usize;
        ok
    }
    /// Palabra clave completa (`and` pero no `android`).
    fn word(&mut self, w: &str) -> bool {
        self.ws();
        let end = self.i + w.len();
        let ok = self.s.get(self.i..end) == Some(w.as_bytes()) && !self.s.get(end).is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_');
        if ok {
            self.i = end;
        }
        ok
    }
    fn or(&mut self) -> Result<Expr, String> {
        let mut a = self.and()?;
        while self.word("or") {
            a = Expr::Or(Box::new(a), Box::new(self.and()?));
        }
        Ok(a)
    }
    fn and(&mut self) -> Result<Expr, String> {
        let mut a = self.not()?;
        while self.word("and") {
            a = Expr::And(Box::new(a), Box::new(self.not()?));
        }
        Ok(a)
    }
    fn not(&mut self) -> Result<Expr, String> {
        if self.word("not") {
            return Ok(Expr::Not(Box::new(self.not()?)));
        }
        self.cmp()
    }
    fn cmp(&mut self) -> Result<Expr, String> {
        let a = self.add()?;
        self.ws();
        let rest = &self.s[self.i.min(self.s.len())..];
        let op = ["==", "!=", "<=", ">=", "<", ">", "="].into_iter().find(|o| rest.starts_with(o.as_bytes()));
        let Some(op) = op else { return Ok(a) };
        self.i += op.len();
        let op = if op == "=" { "==" } else { op };
        Ok(Expr::Cmp(op, Box::new(a), Box::new(self.add()?)))
    }
    fn add(&mut self) -> Result<Expr, String> {
        let mut a = self.mul()?;
        loop {
            let op = if self.eat(b'+') { b'+' } else if self.eat(b'-') { b'-' } else { return Ok(a) };
            a = Expr::Bin(op, Box::new(a), Box::new(self.mul()?));
        }
    }
    fn mul(&mut self) -> Result<Expr, String> {
        let mut a = self.unary()?;
        loop {
            let op = if self.eat(b'*') {
                b'*'
            } else if self.eat(b'/') {
                b'/'
            } else if self.eat(b'%') {
                b'%'
            } else {
                return Ok(a);
            };
            a = Expr::Bin(op, Box::new(a), Box::new(self.unary()?));
        }
    }
    fn unary(&mut self) -> Result<Expr, String> {
        if self.eat(b'-') {
            Ok(Expr::Neg(Box::new(self.unary()?)))
        } else {
            self.atom()
        }
    }
    fn ident(&mut self) -> String {
        self.ws();
        let st = self.i;
        while self.s.get(self.i).is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_') {
            self.i += 1;
        }
        String::from_utf8_lossy(&self.s[st..self.i]).into()
    }
    fn atom(&mut self) -> Result<Expr, String> {
        if self.eat(b'(') {
            let e = self.or()?;
            return if self.eat(b')') { Ok(e) } else { Err("falta «)»".into()) };
        }
        if self.eat(b'"') {
            let st = self.i;
            while self.s.get(self.i).is_some_and(|&c| c != b'"') {
                self.i += 1;
            }
            let txt = String::from_utf8_lossy(&self.s[st..self.i]).into_owned();
            return if self.eat(b'"') { Ok(Expr::Str(txt)) } else { Err("falta cerrar comillas".into()) };
        }
        self.ws();
        if self.s.get(self.i).is_some_and(|c| c.is_ascii_digit() || *c == b'.') {
            let st = self.i;
            while self.s.get(self.i).is_some_and(|c| c.is_ascii_digit() || *c == b'.') {
                self.i += 1;
            }
            let t = std::str::from_utf8(&self.s[st..self.i]).unwrap_or("");
            return t.parse().map(Expr::Num).map_err(|_| format!("número inválido «{t}»"));
        }
        let id = self.ident();
        if id.is_empty() {
            return Err(match self.s.get(self.i) {
                Some(&c) => format!("no esperaba «{}» en la posición {}", c as char, self.i),
                None => "la expresión termina antes de tiempo".into(),
            });
        }
        if self.eat(b'.') {
            let p = self.ident();
            return if p.is_empty() { Err(format!("falta la propiedad después de «{id}.»")) } else { Ok(Expr::Prop(id, p)) };
        }
        if self.eat(b'(') {
            let mut args = vec![];
            if !self.eat(b')') {
                loop {
                    args.push(self.or()?);
                    if self.eat(b')') {
                        break;
                    }
                    if !self.eat(b',') {
                        return Err(format!("falta «,» o «)» en {id}(...)"));
                    }
                }
            }
            return Ok(Expr::Call(id, args));
        }
        Ok(match id.as_str() {
            "true" => Expr::Num(1.),
            "false" => Expr::Num(0.),
            _ => Expr::Var(id),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct C;
    impl Ctx for C {
        fn var(&self, n: &str) -> Value {
            if n == "s" { Value::Num(2.) } else { Value::Str(n.into()) }
        }
        fn prop(&self, o: &str, p: &str) -> Value {
            Value::Str(format!("{o}:{p}"))
        }
        fn call(&self, _: &str, _: &[Expr]) -> Option<Value> {
            None
        }
        fn random(&self) -> f64 {
            0.5
        }
    }

    fn ev(s: &str) -> Value {
        eval(&parse(s).unwrap(), &C)
    }

    #[test]
    fn gramatica() {
        assert_eq!(ev("10 - 4 - 3"), Value::Num(3.)); // izquierda, no 10-(4-3)
        assert_eq!(ev("(1+2)*s - -1"), Value::Num(7.));
        assert_eq!(ev("7 % 3 + 8 / 4"), Value::Num(3.));
        assert_eq!(ev("\"P: \" + s"), Value::Str("P: 2".into()));
        assert_eq!(ev("A.x"), Value::Str("A:x".into()));
        assert_eq!(ev("random(10, 20)"), Value::Num(15.));
        assert_eq!(ev("max(1, min(5, 3))"), Value::Num(3.));
        assert_eq!(ev("3 > 2 and not (1 == 2)"), Value::Num(1.));
        assert_eq!(ev("s >= 2 or 0"), Value::Num(1.));
        assert_eq!(ev("if(s = 2, \"sí\", \"no\")"), Value::Str("sí".into()));
        assert_eq!(ev("\"abc\" < \"abd\""), Value::Num(1.));
        assert_eq!(ev("1 + 1 == 2"), Value::Num(1.)); // la suma va antes que la comparación
        assert_eq!(ev("round(lengthdir_x(10, 90) * 100)"), Value::Num(0.));
        assert_eq!(ev("atan2(1, 0)"), Value::Num(90.));
        assert_eq!(ev("substr(\"hola mundo\", 5)"), Value::Str("mundo".into()));
        assert_eq!(ev("fixed(3.14159, 2) + \" \" + pad(7, 3)"), Value::Str("3.14 007".into()));
        assert_eq!(ev("contains(\"banana\", \"nan\") + sign(-4)"), Value::Num(0.));
        assert_eq!(ev("android"), Value::Str("android".into())); // «and» solo como palabra completa
        assert!(parse("1 +").is_err());
        assert!(parse("f(1 2)").is_err());
        assert!(parse("\"abierta").is_err());
    }
}
