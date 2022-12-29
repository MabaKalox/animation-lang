use crate::instructions;
use crate::program::{Program, SyntaxError};

#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    Expression(Expression),
    Special(instructions::Special),
    UserCall(instructions::UserCommand, Vec<Expression>),
    User(instructions::UserCommand),
    Statements(Vec<Node>),
    Loop(Vec<Node>),
    If(Expression, Vec<Node>),
    IfElse(Expression, Vec<Node>, Vec<Node>),
    NewVarAssignment(String, Expression),
    VarAssignment(String, Expression),
    For(String, Expression, Vec<Node>),
}

#[derive(Debug, Default)]
pub struct Scope<'a> {
    variables: Vec<String>,
    level: u32,
    parent: Option<&'a Scope<'a>>,
}

impl<'a> Scope<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn nest(&'a self) -> Scope<'a> {
        Scope {
            parent: Some(self),
            level: 0,
            variables: vec![],
        }
    }

    pub fn unnest(&mut self, program: &mut Program) -> Result<(), SyntaxError> {
        match self.parent {
            Some(_) => {
                self.assemble_teardown(program)?;
                self.parent = None;
                Ok(())
            }
            None => Err(SyntaxError::ConnotUnnest),
        }
    }

    pub fn index_of(&self, variable_name: &str) -> Option<u32> {
        if let Some(i) = self.variables.iter().position(|r| r == variable_name) {
            Some(self.level - 1 - (i as u32))
        } else if let Some(p) = self.parent {
            p.index_of(variable_name)
                .map(|p_index| p_index + self.level)
        } else {
            None
        }
    }

    pub fn define_variable(&mut self, variable_name: &str) -> Result<(), SyntaxError> {
        if self.variables.iter().any(|r| r == variable_name) {
            return Err(SyntaxError::RedifinedVariable(variable_name.to_string()));
        }

        self.variables.push(variable_name.to_string());

        Ok(())
    }

    pub fn undefine_variable(&mut self, variable_name: &str) -> Result<(), SyntaxError> {
        if let Some(p) = self.variables.iter().position(|r| r == variable_name) {
            self.variables.remove(p);
            Ok(())
        } else {
            Err(SyntaxError::UndefinedVariable(variable_name.to_string()))
        }
    }

    pub(crate) fn assemble_teardown(&self, program: &mut Program) -> Result<(), SyntaxError> {
        if !self.variables.is_empty() {
            program.pop(self.variables.len() as u8)?;
        }

        Ok(())
    }
}

impl Node {
    pub fn assemble(&self, program: &mut Program, scope: &mut Scope) -> Result<(), SyntaxError> {
        match self {
            Node::Expression(e) => {
                e.assemble(program, scope)?;
                program.pop(1)?;
                scope.level -= 1;
            }
            Node::Special(s) => {
                program.special(*s);
            }
            Node::User(s) => {
                program.user(*s);
            }
            Node::UserCall(s, e) => {
                match s {
                    instructions::UserCommand::SET_PIXEL => {
                        let pre_level = scope.level;
                        let mut color_expression = Expression::Binary(
                            Box::new(e[1].clone()),
                            instructions::Binary::AND,
                            Box::new(Expression::Literal(0xFF)),
                        ); // Red

                        for (n, param) in e.iter().enumerate() {
                            if n > 1 {
                                // (param & 0xFF)
                                let mut wrapped = Expression::Binary(
                                    Box::new(param.clone()),
                                    instructions::Binary::AND,
                                    Box::new(Expression::Literal(0xFF)),
                                );

                                // (param & 0xFF) << ((n-1)*8)
                                for _ in 0..(n - 1) {
                                    wrapped = Expression::Unary(
                                        instructions::Unary::SHL8,
                                        Box::new(wrapped),
                                    );
                                }

                                // (color_expression | (param & 0xFF) << ((n-1)*8))
                                color_expression = Expression::Binary(
                                    Box::new(color_expression),
                                    instructions::Binary::OR,
                                    Box::new(wrapped),
                                );
                            }
                        }

                        // Index
                        e[0].assemble(program, scope)?;
                        scope.level = pre_level + 1;
                        color_expression.assemble(program, scope)?;
                        scope.level = pre_level;
                    }
                    _ => {
                        for param in e.iter() {
                            param.assemble(program, scope)?;
                        }
                    }
                }
                program.user(*s);
                program.pop(1)?;
            }
            Node::Statements(stmts) => {
                for i in stmts.iter() {
                    i.assemble(program, scope)?;
                }
            }
            Node::Loop(stmts) => {
                program.repeat_forever(|q| {
                    let mut child_scope = scope.nest();
                    for i in stmts.iter() {
                        i.assemble(q, &mut child_scope)?;
                    }
                    child_scope.unnest(q)?;
                    Ok(())
                })?;
            }
            Node::For(variable_name, expression, stmts) => {
                expression.assemble(program, scope)?;
                scope.define_variable(variable_name)?;
                program.repeat(|q| {
                    let mut child_scope = scope.nest();
                    for i in stmts.iter() {
                        i.assemble(q, &mut child_scope)?;
                    }
                    child_scope.unnest(q)?;

                    Ok(())
                })?;

                // Undefine variable
                scope.undefine_variable(variable_name)?;
                scope.level -= 1;
                program.pop(1)?;
            }
            Node::If(e, ss) => {
                let old_level = scope.level;
                e.assemble(program, scope)?;
                program.if_not_zero(|q| {
                    let mut child_scope = scope.nest();
                    for i in ss.iter() {
                        i.assemble(q, &mut child_scope)?;
                    }
                    child_scope.unnest(q)?;
                    Ok(())
                })?;
                program.pop(1)?;
                scope.level = old_level;
            }
            Node::IfElse(e, if_statements, else_statements) => {
                let old_level = scope.level;
                e.assemble(program, scope)?;
                program.if_not_zero(|q| {
                    let mut child_scope = scope.nest();
                    for i in if_statements.iter() {
                        i.assemble(q, &mut child_scope)?;
                    }
                    child_scope.unnest(q)?;
                    Ok(())
                })?;
                program.if_zero(|q| {
                    let mut child_scope = scope.nest();
                    for i in else_statements.iter() {
                        i.assemble(q, &mut child_scope)?;
                    }
                    child_scope.unnest(q)?;
                    Ok(())
                })?;
                program.pop(1)?;
                scope.level = old_level;
            }
            Node::NewVarAssignment(variable_name, expression) => {
                expression.assemble(program, scope)?;
                scope.define_variable(variable_name)?; // Value left on the stack but cleaned up later by Scope::assemble_teardown
            }
            Node::VarAssignment(variable_name, expression) => {
                let old_level = scope.level;
                expression.assemble(program, scope)?;
                if let Some(relative) = scope.index_of(variable_name) {
                    program.swap((relative) as u8)?;
                    program.pop(1)?;
                } else {
                    return Err(SyntaxError::UndefinedVariable(variable_name.to_string()));
                }
                scope.level = old_level;
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Intrinsic {
    Clamp(Box<Expression>, Box<Expression>, Box<Expression>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Expression {
    Literal(u32),
    Unary(instructions::Unary, Box<Expression>),
    Binary(Box<Expression>, instructions::Binary, Box<Expression>),
    User(instructions::UserCommand),
    UserCall(instructions::UserCommand, Vec<Expression>),
    Load(String),
    Intrinsic(Intrinsic),
}

impl Expression {
    fn assemble(&self, program: &mut Program, scope: &mut Scope) -> Result<(), SyntaxError> {
        // If we can be simplified to a constant expression, do that!
        if let Some(c) = self.const_value() {
            program.push(c);
            scope.level += 1;
            return Ok(());
        }

        match self {
            Expression::Literal(u) => {
                program.push(*u);
                scope.level += 1;
            }
            Expression::User(s) => {
                program.user(*s);
                scope.level += 1;
            }
            Expression::UserCall(s, e) => {
                let old_level = scope.level;
                for param in e.iter() {
                    param.assemble(program, scope)?;
                }
                program.user(*s);
                scope.level = old_level + 1;
            }
            Expression::Unary(op, rhs) => {
                rhs.assemble(program, scope)?;
                program.unary(*op);
            }
            Expression::Binary(lhs, op, rhs) => {
                lhs.assemble(program, scope)?;
                rhs.assemble(program, scope)?;
                program.binary(*op);
                scope.level -= 1;
            }
            Expression::Load(variable_name) => {
                if let Some(relative) = scope.index_of(variable_name) {
                    // println!("Index of {} is {}", variable_name, relative);
                    program.peek(relative as u8)?;
                    scope.level += 1;
                } else {
                    return Err(SyntaxError::UndefinedVariable(variable_name.to_string()));
                }
            }
            Expression::Intrinsic(intrinsic) => {
                match intrinsic {
                    Intrinsic::Clamp(value, min, max) => {
                        let old_level = scope.level;
                        value.assemble(program, scope)?; // [value]
                        min.assemble(program, scope)?; // [min, value]
                        program.peek(1)?; // [value, min, value]
                        program.peek(1)?; // [min, value, min, value]
                        program.binary(instructions::Binary::LT); // [value < min, min, value]

                        // value < min
                        program.if_not_zero(|b| {
                            b.pop(1)?; // [min, value]
                            b.swap(1)?; // [value, min]
                            b.pop(1)?; // [min]
                            b.leave_on_stack(-2);
                            Ok(())
                        })?;

                        // value >= min
                        program.if_zero(|b| {
                            b.pop(2)?; // [value]
                            b.leave_on_stack(-2);
                            Ok(())
                        })?;

                        program.leave_on_stack(2);

                        max.assemble(program, scope)?; // [max, previous_result]
                        program.peek(1)?; // [previous_result, max, previous_result]
                        program.peek(1)?; // [max, previous_result, max, previous_result]
                        program.binary(instructions::Binary::GT); // [previous_result > max, max, previous_result]

                        // previous_result > max
                        program.if_not_zero(|b| {
                            b.pop(1)?; // [max, previous_result]
                            b.swap(1)?; // [previous_result, max]
                            b.pop(1)?; // [max]
                            b.leave_on_stack(-2);
                            Ok(())
                        })?;

                        // previous_result <= max
                        program.if_zero(|b| {
                            b.pop(2)?; // [previous_result]
                            b.leave_on_stack(-2);
                            Ok(())
                        })?;

                        program.leave_on_stack(2);
                        scope.level = old_level + 1;
                    }
                }
            }
        }

        Ok(())
    }

    fn const_value(&self) -> Option<u32> {
        match &self {
            Expression::Literal(u) => Some(*u),
            Expression::UserCall(_, _) | Expression::User(_) => None,
            Expression::Load(_var_name) => None,
            Expression::Binary(lhs, op, rhs) => {
                if let (Some(lhc), Some(rhc)) = (lhs.const_value(), rhs.const_value()) {
                    match op {
                        instructions::Binary::ADD => Some(lhc.overflowing_add(rhc).0),
                        instructions::Binary::SUB => Some(lhc.overflowing_sub(rhc).0),
                        instructions::Binary::DIV => Some(lhc.overflowing_div(rhc).0),
                        instructions::Binary::MUL => Some(lhc.overflowing_mul(rhc).0),
                        instructions::Binary::MOD => Some(lhc % rhc),
                        instructions::Binary::EQ => Some(u32::from(lhc == rhc)),
                        instructions::Binary::NEQ => Some(u32::from(lhc != rhc)),
                        instructions::Binary::LT => Some(u32::from(lhc < rhc)),
                        instructions::Binary::LTE => Some(u32::from(lhc <= rhc)),
                        instructions::Binary::GT => Some(u32::from(lhc > rhc)),
                        instructions::Binary::GTE => Some(u32::from(lhc >= rhc)),
                        instructions::Binary::OR => Some(lhc | rhc),
                        instructions::Binary::XOR => Some(lhc ^ rhc),
                        instructions::Binary::AND => Some(lhc & rhc),
                        instructions::Binary::SHL => Some(lhc << rhc),
                        instructions::Binary::SHR => Some(lhc >> rhc),
                    }
                } else {
                    None
                }
            }

            Expression::Unary(op, rhs) => {
                if let Some(c) = rhs.const_value() {
                    match op {
                        instructions::Unary::INC => Some(c.overflowing_add(1).0),
                        instructions::Unary::DEC => Some(c.overflowing_sub(1).0),
                        instructions::Unary::NOT => Some(!c),
                        instructions::Unary::NEG => None, // TODO
                        instructions::Unary::SHL8 => Some(c << 8),
                        instructions::Unary::SHR8 => Some(c << 8),
                    }
                } else {
                    None
                }
            }

            Expression::Intrinsic(intrinsic) => {
                match intrinsic {
                    Intrinsic::Clamp(value, min, max) => {
                        // When all parameters are constant we don't have to think long
                        if let (Some(c_value), Some(c_min), Some(c_max)) =
                            (value.const_value(), min.const_value(), max.const_value())
                        {
                            let mut result = c_value;
                            if result < c_min {
                                result = c_min;
                            }
                            if result > c_max {
                                result = c_max;
                            }
                            Some(result)
                        } else {
                            None
                        }
                    }
                }
            }
        }
    }
}
