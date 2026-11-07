//! Expresiones: `Player.x + 10`, `"Puntos: " + score`, `random(0, 100)`, `count(Coin)`.
//! Operadores + - * / % con asociatividad izquierda (a-b-c = (a-b)-c).

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
    let e = p.add()?;
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
        Expr::Bin(op, a, b) => {
            let (a, b) = (eval(a, c), eval(b, c));
            if *op == b'+' && (matches!(a, Value::Str(_)) || matches!(b, Value::Str(_))) {
                return Value::Str(a.text() + &b.text());
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
            let e = self.add()?;
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
                    args.push(self.add()?);
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
        assert!(parse("1 +").is_err());
        assert!(parse("f(1 2)").is_err());
        assert!(parse("\"abierta").is_err());
    }
}
