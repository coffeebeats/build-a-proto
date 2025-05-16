use chumsky::prelude::*;

#[derive(Debug)]
enum Expr<'a> {
    Num(f64),
    Var(&'a str),

    Neg(Box<Expr<'a>>),
    Add(Box<Expr<'a>>, Box<Expr<'a>>),
    Sub(Box<Expr<'a>>, Box<Expr<'a>>),
    Mul(Box<Expr<'a>>, Box<Expr<'a>>),
    Div(Box<Expr<'a>>, Box<Expr<'a>>),

    Call(&'a str, Vec<Expr<'a>>),
    Let {
        name: &'a str,
        rhs: Box<Expr<'a>>,
        then: Box<Expr<'a>>,
    },
    Fn {
        name: &'a str,
        args: Vec<&'a str>,
        body: Box<Expr<'a>>,
        then: Box<Expr<'a>>,
    },
}

pub fn parse(input: &str) -> anyhow::Result<()> {
    println!("{}", input);
    Ok(())
}
