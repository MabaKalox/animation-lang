pub mod ast;

use nom::{
    branch::alt,
    bytes::complete::{is_not, tag, take_while, take_while1},
    combinator::{map, map_res, opt},
    multi::{fold_many0, separated_list0},
    sequence::{delimited, pair, preceded, terminated, tuple},
    Finish, IResult,
};

use crate::program::Program;
use crate::{instructions, program::SyntaxError};
use ast::{Expression, Intrinsic, Node, Scope};

fn from_hex(input: &str) -> Result<u32, std::num::ParseIntError> {
    u32::from_str_radix(input, 16)
}

fn from_dec(input: &str) -> Result<u32, std::num::ParseIntError> {
    input.parse::<u32>()
}

fn is_hex_digit(c: char) -> bool {
    c.is_ascii_hexdigit()
}

fn is_dec_digit(c: char) -> bool {
    c.is_ascii_digit()
}

fn whitespace(input: &str) -> IResult<&str, &str> {
    let chars = " \t\r\n ";
    take_while(move |c| chars.contains(c))(input)
}

fn sp(input: &str) -> IResult<&str, ()> {
    let mut input = input;
    loop {
        let (j, x) = whitespace(input)?;
        let (j, y) = opt(comment)(j)?;
        let (j, z) = whitespace(j)?;
        if x.is_empty() && y.is_none() && z.is_empty() {
            break;
        }
        input = j;
    }
    Ok((input, ()))
}

fn hex_number(input: &str) -> IResult<&str, u32> {
    map_res(take_while1(is_hex_digit), from_hex)(input)
}

fn dec_number(input: &str) -> IResult<&str, u32> {
    map_res(take_while1(is_dec_digit), from_dec)(input)
}

fn variable_name(input: &str) -> IResult<&str, &str> {
    take_while1(|c: char| c.is_alphabetic())(input)
}

fn hex_literal(input: &str) -> IResult<&str, u32> {
    let (input, _) = tag("0x")(input)?;
    let (input, num) = hex_number(input)?;
    Ok((input, num))
}

fn literal(input: &str) -> IResult<&str, Expression> {
    let (input, res) = alt((hex_literal, dec_number))(input)?;
    Ok((input, Expression::Literal(res)))
}

fn load_expression(input: &str) -> IResult<&str, Expression> {
    map(variable_name, |v| Expression::Load(v.to_string()))(input)
}

fn bracketed_expression(input: &str) -> IResult<&str, Expression> {
    preceded(tag("("), terminated(expression, tag(")")))(input)
}

fn term(input: &str) -> IResult<&str, Expression> {
    alt((
        literal,
        user_expression,
        load_expression,
        bracketed_expression,
    ))(input)
}

fn comparison(input: &str) -> IResult<&str, Expression> {
    let (input, init) = unaries(input)?;

    #[allow(clippy::needless_return)]
    return fold_many0(
        pair(
            preceded(
                sp,
                terminated(
                    alt((
                        tag(">="),
                        tag("<="),
                        tag(">"),
                        tag("<"),
                        tag("=="),
                        tag("!="),
                    )),
                    sp,
                ),
            ),
            unaries,
        ),
        || init.clone(),
        |acc, (op, val): (&str, Expression)| match op {
            ">=" => Expression::Binary(Box::new(acc), instructions::Binary::GTE, Box::new(val)),
            "<=" => Expression::Binary(Box::new(acc), instructions::Binary::LTE, Box::new(val)),
            ">" => Expression::Binary(Box::new(acc), instructions::Binary::GT, Box::new(val)),
            "<" => Expression::Binary(Box::new(acc), instructions::Binary::LT, Box::new(val)),
            "==" => Expression::Binary(Box::new(acc), instructions::Binary::EQ, Box::new(val)),
            "!=" => Expression::Binary(Box::new(acc), instructions::Binary::NEQ, Box::new(val)),
            _ => unreachable!(),
        },
    )(input);
}

fn unaries(input: &str) -> IResult<&str, Expression> {
    alt((
        map(pair(alt((tag("-"), tag("~"))), unaries), |t| match t.0 {
            "-" => Expression::Unary(instructions::Unary::NEG, Box::new(t.1)),
            "~" => Expression::Unary(instructions::Unary::NOT, Box::new(t.1)),
            _ => unreachable!(),
        }),
        binaries,
    ))(input)
}

fn binaries(input: &str) -> IResult<&str, Expression> {
    let (input, init) = addition(input)?;

    #[allow(clippy::needless_return)]
    return fold_many0(
        pair(
            terminated(
                preceded(
                    sp,
                    alt((tag("|"), tag("^"), tag("&"), tag(">>"), tag("<<"))),
                ),
                sp,
            ),
            addition,
        ),
        || init.clone(),
        |acc, (op, val): (&str, Expression)| match op {
            "&" => Expression::Binary(Box::new(acc), instructions::Binary::AND, Box::new(val)),
            "|" => Expression::Binary(Box::new(acc), instructions::Binary::OR, Box::new(val)),
            "^" => Expression::Binary(Box::new(acc), instructions::Binary::XOR, Box::new(val)),
            ">>" => Expression::Binary(Box::new(acc), instructions::Binary::SHR, Box::new(val)),
            "<<" => Expression::Binary(Box::new(acc), instructions::Binary::SHL, Box::new(val)),
            _ => unreachable!(),
        },
    )(input);
}

fn addition(input: &str) -> IResult<&str, Expression> {
    let (input, init) = multiplication(input)?;

    #[allow(clippy::needless_return)]
    return fold_many0(
        pair(
            terminated(preceded(sp, alt((tag("+"), tag("-")))), sp),
            multiplication,
        ),
        || init.clone(),
        |acc, (op, val): (&str, Expression)| {
            if op == "+" {
                Expression::Binary(Box::new(acc), instructions::Binary::ADD, Box::new(val))
            } else {
                Expression::Binary(Box::new(acc), instructions::Binary::SUB, Box::new(val))
            }
        },
    )(input);
}

fn multiplication(input: &str) -> IResult<&str, Expression> {
    let (input, init) = term(input)?;

    #[allow(clippy::needless_return)]
    return fold_many0(
        pair(
            terminated(
                preceded(
                    sp,
                    alt((tag("*"), tag("/"), tag("%"), tag("<<"), tag(">>"))),
                ),
                sp,
            ),
            term,
        ),
        || init.clone(),
        |acc, (op, val): (&str, Expression)| match op {
            "*" => Expression::Binary(Box::new(acc), instructions::Binary::MUL, Box::new(val)),
            "/" => Expression::Binary(Box::new(acc), instructions::Binary::DIV, Box::new(val)),
            "%" => Expression::Binary(Box::new(acc), instructions::Binary::MOD, Box::new(val)),
            "<<" | ">>" => {
                let binary_op = match op {
                    "<<" => instructions::Binary::SHL,
                    ">>" => instructions::Binary::SHR,
                    _ => unreachable!(),
                };

                if let Expression::Literal(n) = val {
                    let unary = match op {
                        "<<" => instructions::Unary::SHL8,
                        ">>" => instructions::Unary::SHR8,
                        _ => unreachable!(),
                    };

                    if (n % 8) == 0 {
                        let times = n / 8;
                        let mut expr = acc;
                        for _ in 0..times {
                            expr = Expression::Unary(unary, Box::new(expr))
                        }
                        expr
                    } else {
                        Expression::Binary(Box::new(acc), binary_op, Box::new(val))
                    }
                } else {
                    Expression::Binary(Box::new(acc), binary_op, Box::new(val))
                }
            }
            _ => unreachable!(),
        },
    )(input);
}

fn expression(input: &str) -> IResult<&str, Expression> {
    comparison(input)
}

fn expression_statement(input: &str) -> IResult<&str, Node> {
    map(expression, Node::Expression)(input)
}

fn special_statement(input: &str) -> IResult<&str, Node> {
    map(tag("dump"), |_| Node::Special(instructions::Special::DUMP))(input)
}

fn user_statement(input: &str) -> IResult<&str, Node> {
    alt((
        map(tag("blit"), |_| Node::User(instructions::UserCommand::BLIT)),
        // set_pixel(i, r, g, b)
        map(
            tuple((
                tag("set_pixel("),
                preceded(sp, terminated(expression, sp)),
                tag(","),
                preceded(sp, terminated(expression, sp)),
                tag(","),
                preceded(sp, terminated(expression, sp)),
                tag(","),
                preceded(sp, terminated(expression, sp)),
                tag(","),
                preceded(sp, terminated(expression, sp)),
                tag(")"),
            )),
            |t| {
                Node::UserCall(
                    instructions::UserCommand::SET_PIXEL,
                    vec![t.1, t.3, t.5, t.7, t.9],
                )
            },
        ),
    ))(input)
}

fn user_expression(input: &str) -> IResult<&str, Expression> {
    alt((
        map(tuple((tag("random("), expression, tag(")"))), |t| {
            Expression::UserCall(instructions::UserCommand::RANDOM_INT, vec![t.1])
        }),
        map(tuple((tag("get_pixel("), expression, tag(")"))), |t| {
            Expression::UserCall(instructions::UserCommand::GET_PIXEL, vec![t.1])
        }),
        map(tag("get_length"), |_| {
            Expression::User(instructions::UserCommand::GET_LENGTH)
        }),
        map(tag("get_wall_time"), |_| {
            Expression::User(instructions::UserCommand::GET_WALL_TIME)
        }),
        map(tag("get_precise_time"), |_| {
            Expression::User(instructions::UserCommand::GET_PRECISE_TIME)
        }),
        /* Compiler intrinsics: 'functions' that simply compile to an expression  */
        // rgb(r, g, b) => color value (0xBBGGRRII)
        map(
            tuple((
                tag("rgb("),
                preceded(sp, terminated(expression, sp)),
                tag(","),
                preceded(sp, terminated(expression, sp)),
                tag(","),
                preceded(sp, terminated(expression, sp)),
                tag(")"),
            )),
            |t| {
                // (r & 0xFF) | (g & 0xFF) << 8 | (b & 0xFF) << 16
                let vals = vec![t.3, t.5];
                let mut shift: u32 = 8;
                let mut root: Box<Expression> = Box::new(Expression::Binary(
                    Box::new(t.1),
                    instructions::Binary::AND,
                    Box::new(Expression::Literal(0xFF)),
                ));

                for val in vals {
                    root = Box::new(Expression::Binary(
                        root,
                        instructions::Binary::OR,
                        Box::new(Expression::Binary(
                            Box::new(Expression::Binary(
                                Box::new(val),
                                instructions::Binary::AND,
                                Box::new(Expression::Literal(0xFF)),
                            )),
                            instructions::Binary::SHL,
                            Box::new(Expression::Literal(shift)),
                        )),
                    ));
                    shift += 8;
                }
                *root
            },
        ),
        // clamp(value, min, max):
        map(
            tuple((
                tag("clamp("),
                preceded(sp, terminated(expression, sp)),
                tag(","),
                preceded(sp, terminated(expression, sp)),
                tag(","),
                preceded(sp, terminated(expression, sp)),
                tag(")"),
            )),
            |t| {
                Expression::Intrinsic(Intrinsic::Clamp(
                    Box::new(t.1),
                    Box::new(t.3),
                    Box::new(t.5),
                ))
            },
        ),
        //red(color)
        map(tuple((tag("red("), expression, tag(")"))), |t| {
            // x 0xFF
            Expression::Binary(
                Box::new(t.1),
                instructions::Binary::AND,
                Box::new(Expression::Literal(0xFF)),
            )
        }),
        map(tuple((tag("green("), expression, tag(")"))), |t| {
            // (x >> 8) & 0xFF
            Expression::Binary(
                Box::new(Expression::Unary(instructions::Unary::SHR8, Box::new(t.1))),
                instructions::Binary::AND,
                Box::new(Expression::Literal(0xFF)),
            )
        }),
        map(tuple((tag("blue("), expression, tag(")"))), |t| {
            // (x >> 16) & 0xFF
            Expression::Binary(
                Box::new(Expression::Unary(
                    instructions::Unary::SHR8,
                    Box::new(Expression::Unary(instructions::Unary::SHR8, Box::new(t.1))),
                )),
                instructions::Binary::AND,
                Box::new(Expression::Literal(0xFF)),
            )
        }),
    ))(input)
}

fn if_statement(input: &str) -> IResult<&str, Node> {
    map(
        tuple((
            tag("if("),
            preceded(sp, terminated(expression, sp)),
            tag(")"),
            sp,
            tag("{"),
            sp,
            program,
            sp,
            tag("}"),
            sp,
            opt(tuple((tag("else {"), sp, program, sp, tag("}"), sp))),
        )),
        |t| {
            if let Node::Statements(if_statements) = t.6 {
                if let Some(else_tuple) = t.10 {
                    if let Node::Statements(else_statements) = else_tuple.2 {
                        Node::IfElse(t.1, if_statements, else_statements)
                    } else {
                        unreachable!()
                    }
                } else {
                    Node::If(t.1, if_statements)
                }
            } else {
                unreachable!()
            }
        },
    )(input)
}

fn loop_statement(input: &str) -> IResult<&str, Node> {
    map(
        tuple((tag("loop"), sp, tag("{"), sp, program, tag("}"))),
        |t| {
            if let Node::Statements(ss) = t.4 {
                Node::Loop(ss)
            } else {
                unreachable!()
            }
        },
    )(input)
}

fn comment(input: &str) -> IResult<&str, &str> {
    alt((multi_line_comment, single_line_comment))(input)
}

fn multi_line_comment(input: &str) -> IResult<&str, &str> {
    delimited(tag("/*"), is_not("*/"), tag("*/"))(input)
}

fn single_line_comment(input: &str) -> IResult<&str, &str> {
    delimited(tag("//"), is_not("\n"), tag("\n"))(input)
}

fn for_statement(input: &str) -> IResult<&str, Node> {
    map(
        tuple((
            tag("for("),
            preceded(sp, terminated(variable_name, sp)),
            tag("="),
            preceded(sp, terminated(expression, sp)),
            tag(")"),
            sp,
            tag("{"),
            sp,
            program,
            sp,
            tag("}"),
        )),
        |t| {
            if let Node::Statements(ss) = t.8 {
                Node::For(t.1.to_string(), t.3, ss)
            } else {
                unreachable!()
            }
        },
    )(input)
}

fn new_var_assigment_statement(input: &str) -> IResult<&str, Node> {
    map(
        tuple((
            terminated(tag("let"), sp),
            variable_name,
            preceded(sp, terminated(tag("="), sp)),
            expression,
        )),
        |t| Node::NewVarAssignment(t.1.to_string(), t.3),
    )(input)
}

fn var_assigment_statement(input: &str) -> IResult<&str, Node> {
    map(
        tuple((
            variable_name,
            preceded(sp, terminated(tag("="), sp)),
            expression,
        )),
        |t| Node::VarAssignment(t.0.to_string(), t.2),
    )(input)
}

fn statement(input: &str) -> IResult<&str, Node> {
    terminated(
        preceded(
            sp,
            alt((
                user_statement,
                special_statement,
                new_var_assigment_statement,
                var_assigment_statement,
                if_statement,
                for_statement,
                loop_statement,
                expression_statement,
            )),
        ),
        sp,
    )(input)
}

fn program(input: &str) -> IResult<&str, Node> {
    terminated(
        terminated(
            terminated(
                map(
                    separated_list0(preceded(sp, tag(";")), preceded(sp, statement)),
                    Node::Statements,
                ),
                sp,
            ),
            opt(tag(";")),
        ),
        sp,
    )(input)
}

pub trait FromSource {
    fn from_source(source: &str) -> Result<Program, SyntaxError>;
}

impl FromSource for Program {
    fn from_source(source: &str) -> Result<Program, SyntaxError> {
        match program(source).finish() {
            Ok((remainder, n)) => {
                if !remainder.is_empty() {
                    Err(SyntaxError::CouldNotParseRamainder(remainder.to_string()))
                } else {
                    let mut p = Program::new();
                    let mut scope = Scope::new();
                    n.assemble(&mut p, &mut scope)?;
                    scope.assemble_teardown(&mut p)?;
                    Ok(p)
                }
            }
            Err(x) => Err(SyntaxError::ParseError(x.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_decimal_literal() {
        assert_eq!(expression("0x0000CC"), Ok(("", Expression::Literal(204))));
    }

    #[test]
    fn check_hex_literal() {
        assert_eq!(expression("1337"), Ok(("", Expression::Literal(1337))));
    }

    #[test]
    fn check_parsing_addition() {
        assert_eq!(
            expression("1+2"),
            Ok((
                "",
                Expression::Binary(
                    Box::new(Expression::Literal(1)),
                    instructions::Binary::ADD,
                    Box::new(Expression::Literal(2))
                )
            ))
        );
    }

    #[test]
    fn check_parsing_multiplication() {
        assert_eq!(
            expression("1*2"),
            Ok((
                "",
                Expression::Binary(
                    Box::new(Expression::Literal(1)),
                    instructions::Binary::MUL,
                    Box::new(Expression::Literal(2))
                )
            ))
        );
    }

    #[test]
    fn check_parsing_subtraction() {
        assert_eq!(
            expression("1-2"),
            Ok((
                "",
                Expression::Binary(
                    Box::new(Expression::Literal(1)),
                    instructions::Binary::SUB,
                    Box::new(Expression::Literal(2))
                )
            ))
        );
    }

    #[test]
    fn check_parsing_division() {
        assert_eq!(
            expression("1/2"),
            Ok((
                "",
                Expression::Binary(
                    Box::new(Expression::Literal(1)),
                    instructions::Binary::DIV,
                    Box::new(Expression::Literal(2))
                )
            ))
        );
    }

    #[test]
    fn check_parsing_mod() {
        assert_eq!(
            expression("1%2"),
            Ok((
                "",
                Expression::Binary(
                    Box::new(Expression::Literal(1)),
                    instructions::Binary::MOD,
                    Box::new(Expression::Literal(2))
                )
            ))
        );
    }

    #[test]
    fn check_compiler_basic_program() {
        if let Ok((remainder, n)) = program("loop{if(1+2*3>4){blit;};\ndump}") {
            assert_eq!(remainder, "");
            let mut program = Program::new();
            let mut scope = Scope::new();
            n.assemble(&mut program, &mut scope).unwrap();
            scope.assemble_teardown(&mut program).unwrap();
        }
    }

    #[test]
    #[should_panic]
    fn check_undefined_variable() {
        let (remainder, _) = program("loop{some_undefined_variable;};").unwrap();
        assert_eq!(remainder, "");
    }

    #[test]
    #[should_panic]
    fn check_not_terminated_line() {
        let (remainder, _) = program("loop{let a=1+1\n1+2};").unwrap();
        assert_eq!(remainder, "");
    }
}
