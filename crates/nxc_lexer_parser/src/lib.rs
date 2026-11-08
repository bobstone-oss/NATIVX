pub mod ast;
pub mod lexer;

use ast::*;
use lexer::Token;
use logos::Logos;
use std::collections::HashMap;

pub struct NxParser {
    tokens: Vec<Token>,
    pos: usize,
}

impl NxParser {
    pub fn new(tokens: Vec<Token>) -> Self { Self { tokens, pos: 0 } }
    fn peek(&self) -> Option<&Token> { self.tokens.get(self.pos) }
    fn advance(&mut self) -> Option<Token> {
        if self.pos < self.tokens.len() {
            let t = self.tokens[self.pos].clone();
            self.pos += 1;
            Some(t)
        } else { None }
    }
    fn expect(&mut self, expected: &Token) -> Result<(), String> {
        match self.peek() {
            Some(t) if t == expected => { self.advance(); Ok(()) }
            other => Err(format!("Esperado {:?}, encontrado {:?}", expected, other)),
        }
    }
    fn expect_ident(&mut self) -> Result<String, String> {
        match self.advance() {
            Some(Token::Ident(s)) => Ok(s),
            Some(Token::TypeVec2) => Ok("vec2".into()),
            Some(Token::TypeVec3) => Ok("vec3".into()),
            Some(Token::TypeVec4) => Ok("vec4".into()),
            Some(Token::TypeMat4) => Ok("mat4".into()),
            Some(Token::TypeQuat) => Ok("quat".into()),
            Some(Token::TypeWorld) => Ok("World".into()),
            Some(Token::TypeEntityId) => Ok("EntityId".into()),
            Some(Token::TypeCommandBuffer) => Ok("CommandBuffer".into()),
            other => Err(format!("Esperado ident, encontrado {:?}", other)),
        }
    }

    pub fn parse_program(&mut self) -> Result<NxProgram, String> {
        let mut imports = Vec::new();
        let mut structs = Vec::new();
        let mut functions = Vec::new();
        while let Some(tok) = self.peek() {
            match tok {
                Token::Use => {
                    self.advance();
                    while self.peek() != Some(&Token::Semicolon) && self.peek().is_some() {
                        self.advance();
                    }
                    self.expect(&Token::Semicolon)?;
                    imports.push(vec![]);
                }
                Token::AttrGpuShared | Token::AttrComponent | Token::Struct => structs.push(self.parse_struct()?),
                Token::Pub => { self.advance(); }
                _ => functions.push(self.parse_function()?),
            }
        }
        Ok(NxProgram { imports, structs, functions })
    }

    fn parse_struct(&mut self) -> Result<StructDef, String> {
        let mut gpu_shared = false;
        let mut is_component = false;
        while matches!(self.peek(), Some(Token::AttrGpuShared) | Some(Token::AttrComponent)) {
            match self.advance().unwrap() {
                Token::AttrGpuShared => gpu_shared = true,
                Token::AttrComponent => is_component = true,
                _ => {}
            }
        }
        if self.peek() == Some(&Token::Pub) { self.advance(); }
        self.expect(&Token::Struct)?;
        let name = self.expect_ident()?;
        self.expect(&Token::LBrace)?;
        let mut fields = Vec::new();
        while self.peek() != Some(&Token::RBrace) {
            let fname = self.expect_ident()?;
            self.expect(&Token::Colon)?;
            let ftype = self.parse_type()?;
            fields.push((fname, ftype));
            if self.peek() == Some(&Token::Comma) { self.advance(); }
        }
        self.expect(&Token::RBrace)?;
        Ok(StructDef { name, gpu_shared, is_component, fields })
    }

    fn parse_type(&mut self) -> Result<NxType, String> {
        if self.peek() == Some(&Token::RefMut) {
            self.advance();
            return Ok(NxType::MutRef(Box::new(self.parse_type()?)));
        }
        if self.peek() == Some(&Token::Ref) {
            self.advance();
            return Ok(NxType::Ref(Box::new(self.parse_type()?)));
        }
        if self.peek() == Some(&Token::LBracket) {
            self.advance();
            let inner = self.parse_type()?;
            self.expect(&Token::Semicolon)?;
            let Some(Token::IntLit(n)) = self.advance() else { return Err("Longitud inválida".into()); };
            self.expect(&Token::RBracket)?;
            return Ok(NxType::Array(Box::new(inner), n as usize));
        }
        match self.advance() {
            Some(Token::TypeF32) => Ok(NxType::F32),
            Some(Token::TypeI32) => Ok(NxType::I32),
            Some(Token::TypeU32) => Ok(NxType::U32),
            Some(Token::TypeBool) => Ok(NxType::Bool),
            Some(Token::TypeStr) => Ok(NxType::Str),
            Some(Token::TypeVec2) => Ok(NxType::Vec2),
            Some(Token::TypeVec3) => Ok(NxType::Vec3),
            Some(Token::TypeVec4) => Ok(NxType::Vec4),
            Some(Token::TypeMat4) => Ok(NxType::Mat4),
            Some(Token::TypeQuat) => Ok(NxType::Quat),
            Some(Token::TypeWorld) => Ok(NxType::World),
            Some(Token::TypeEntityId) => Ok(NxType::EntityId),
            Some(Token::TypeCommandBuffer) => Ok(NxType::CommandBuffer),
            Some(Token::Ident(s)) => Ok(NxType::from_name(&s)),
            other => Err(format!("Tipo inválido {:?}", other)),
        }
    }

    fn parse_function(&mut self) -> Result<FnDef, String> {
        let decorator = match self.peek() {
            Some(Token::AttrEntry) => { self.advance(); FnDecorator::Entry }
            Some(Token::AttrSystem) => { self.advance(); FnDecorator::System }
            Some(Token::AttrGpuVertex) => { self.advance(); FnDecorator::GpuVertex }
            Some(Token::AttrGpuFragment) => { self.advance(); FnDecorator::GpuFragment }
            Some(Token::AttrGpuCompute) => { self.advance(); FnDecorator::GpuCompute }
            _ => FnDecorator::None,
        };
        if self.peek() == Some(&Token::Pub) { self.advance(); }
        let is_async = if self.peek() == Some(&Token::Async) { self.advance(); true } else { false };
        self.expect(&Token::Fn)?;
        let name = self.expect_ident()?;
        self.expect(&Token::LParen)?;
        let mut params = Vec::new();
        while self.peek() != Some(&Token::RParen) {
            let pn = self.expect_ident()?;
            self.expect(&Token::Colon)?;
            let pt = self.parse_type()?;
            params.push((pn, pt));
            if self.peek() == Some(&Token::Comma) { self.advance(); }
        }
        self.expect(&Token::RParen)?;
        let return_type = if self.peek() == Some(&Token::Arrow) {
            self.advance();
            self.parse_type()?
        } else { NxType::Void };
        let body = self.parse_block()?;
        Ok(FnDef { name, decorator, is_async, params, return_type, body })
    }

    fn parse_block(&mut self) -> Result<Vec<Stmt>, String> {
        self.expect(&Token::LBrace)?;
        let mut stmts = Vec::new();
        while self.peek() != Some(&Token::RBrace) && self.peek().is_some() {
            stmts.push(self.parse_stmt()?);
        }
        self.expect(&Token::RBrace)?;
        Ok(stmts)
    }

    fn parse_stmt(&mut self) -> Result<Stmt, String> {
        match self.peek() {
            Some(Token::Let) | Some(Token::Var) | Some(Token::Temp) | Some(Token::Const) => {
                let kind = match self.advance().unwrap() {
                    Token::Let => VarKind::Let, Token::Var => VarKind::Var,
                    Token::Temp => VarKind::Temp, _ => VarKind::Const,
                };
                let name = self.expect_ident()?;
                let ty = if self.peek() == Some(&Token::Colon) { self.advance(); Some(self.parse_type()?) } else { None };
                self.expect(&Token::Assign)?;
                let value = self.parse_expr()?;
                self.expect(&Token::Semicolon)?;
                Ok(Stmt::VarDecl { kind, name, ty, value })
            }
            Some(Token::If) => {
                self.advance();
                let condition = self.parse_expr()?;
                let then_branch = self.parse_block()?;
                let else_branch = if self.peek() == Some(&Token::Else) {
                    self.advance();
                    Some(self.parse_block()?)
                } else { None };
                Ok(Stmt::IfStmt { condition, then_branch, else_branch })
            }
            Some(Token::For) => {
                self.advance();
                if self.peek() == Some(&Token::LParen) {
                    self.advance();
                    let entity_var = self.expect_ident()?;
                    self.expect(&Token::Comma)?;
                    let mut component_vars = Vec::new();
                    while self.peek() != Some(&Token::RParen) {
                        component_vars.push(self.expect_ident()?);
                        if self.peek() == Some(&Token::Comma) { self.advance(); }
                    }
                    self.expect(&Token::RParen)?;
                    self.expect(&Token::In)?;
                    let q_expr = self.parse_expr()?;
                    if let Expr::MethodCall { receiver, method, args } = q_expr {
                        let mut component_types = Vec::new();
                        for (_, a) in args {
                            if let Expr::Ident(tn) = a { component_types.push(NxType::from_name(&tn)); }
                        }
                        let query_mode = if component_types.len() == 1 { EcsQueryMode::SingleMut }
                            else if method == "query_mut" { EcsQueryMode::PairMut }
                            else { EcsQueryMode::MutRef };
                        let body = self.parse_block()?;
                        return Ok(Stmt::ForEcsQueryStmt {
                            world_expr: *receiver, query_mode, entity_var, component_vars, component_types, body
                        });
                    }
                    return Err("Consulta ECS inválida".into());
                }
                let iterator = self.expect_ident()?;
                self.expect(&Token::In)?;
                let start = self.parse_expr()?;
                self.expect(&Token::DotDot)?;
                let end = self.parse_expr()?;
                let body = self.parse_block()?;
                Ok(Stmt::ForRangeStmt { iterator, start, end, body })
            }
            Some(Token::Return) => {
                self.advance();
                if self.peek() == Some(&Token::Semicolon) { self.advance(); Ok(Stmt::ReturnStmt(None)) }
                else { let e = self.parse_expr()?; self.expect(&Token::Semicolon)?; Ok(Stmt::ReturnStmt(Some(e))) }
            }
            _ => {
                let e = self.parse_expr()?;
                self.expect(&Token::Semicolon)?;
                Ok(Stmt::ExprStmt(e))
            }
        }
    }

    fn parse_expr(&mut self) -> Result<Expr, String> { self.parse_assignment() }

    fn parse_assignment(&mut self) -> Result<Expr, String> {
        let left = self.parse_comparison()?;
        if matches!(self.peek(), Some(Token::Assign) | Some(Token::PlusAssign) | Some(Token::MinusAssign) | Some(Token::MulAssign)) {
            let op = match self.advance().unwrap() {
                Token::Assign => BinOp::Assign, Token::PlusAssign => BinOp::AddAssign,
                Token::MinusAssign => BinOp::SubAssign, _ => BinOp::MulAssign,
            };
            let right = self.parse_assignment()?;
            return Ok(Expr::Binary { left: Box::new(left), op, right: Box::new(right) });
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_additive()?;
        while matches!(self.peek(), Some(Token::EqEq) | Some(Token::Less) | Some(Token::Greater)) {
            let op = match self.advance().unwrap() {
                Token::EqEq => BinOp::Eq, Token::Less => BinOp::Less, _ => BinOp::Greater,
            };
            let right = self.parse_additive()?;
            left = Expr::Binary { left: Box::new(left), op, right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_multiplicative()?;
        while matches!(self.peek(), Some(Token::Plus) | Some(Token::Minus)) {
            let op = if self.advance() == Some(Token::Plus) { BinOp::Add } else { BinOp::Sub };
            let right = self.parse_multiplicative()?;
            left = Expr::Binary { left: Box::new(left), op, right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_unary()?;
        while matches!(self.peek(), Some(Token::Star) | Some(Token::Slash)) {
            let op = if self.advance() == Some(Token::Star) { BinOp::Mul } else { BinOp::Div };
            let right = self.parse_unary()?;
            left = Expr::Binary { left: Box::new(left), op, right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        if self.peek() == Some(&Token::Minus) {
            self.advance();
            return Ok(Expr::Unary { op: UnaryOp::Neg, expr: Box::new(self.parse_unary()?) });
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_primary()?;
        loop {
            if self.peek() == Some(&Token::Dot) {
                self.advance();
                let member = self.expect_ident()?;
                if self.peek() == Some(&Token::LParen) {
                    let args = self.parse_args()?;
                    expr = Expr::MethodCall { receiver: Box::new(expr), method: member, args };
                } else {
                    expr = Expr::FieldAccess { base: Box::new(expr), field: member };
                }
            } else if self.peek() == Some(&Token::LBracket) {
                self.advance();
                let idx = self.parse_expr()?;
                self.expect(&Token::RBracket)?;
                expr = Expr::IndexAccess { array: Box::new(expr), index: Box::new(idx) };
            } else { break; }
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        match self.peek().cloned() {
            Some(Token::FloatLit(f)) => { self.advance(); Ok(Expr::FloatLit(f)) }
            Some(Token::IntLit(i)) => { self.advance(); Ok(Expr::IntLit(i)) }
            Some(Token::LParen) => { self.advance(); let e = self.parse_expr()?; self.expect(&Token::RParen)?; Ok(e) }
            Some(Token::LBracket) => {
                self.advance();
                let mut v = Vec::new();
                while self.peek() != Some(&Token::RBracket) {
                    v.push(self.parse_expr()?);
                    if self.peek() == Some(&Token::Comma) { self.advance(); }
                }
                self.expect(&Token::RBracket)?;
                Ok(Expr::ArrayLit(v))
            }
            _ => {
                let name = self.expect_ident()?;
                if self.peek() == Some(&Token::LParen) {
                    Ok(Expr::FnCall { name, args: self.parse_args()? })
                } else if self.peek() == Some(&Token::LBrace)
                    && matches!(self.tokens.get(self.pos + 1), Some(Token::Ident(_)))
                    && matches!(self.tokens.get(self.pos + 2), Some(Token::Colon))
                {
                    self.advance();
                    let mut fields = Vec::new();
                    while self.peek() != Some(&Token::RBrace) {
                        let k = self.expect_ident()?;
                        self.expect(&Token::Colon)?;
                        let v = self.parse_expr()?;
                        fields.push((k, v));
                        if self.peek() == Some(&Token::Comma) { self.advance(); }
                    }
                    self.expect(&Token::RBrace)?;
                    Ok(Expr::StructInit { name, fields })
                } else {
                    Ok(Expr::Ident(name))
                }
            }
        }
    }

    fn parse_args(&mut self) -> Result<Vec<(Option<String>, Expr)>, String> {
        self.expect(&Token::LParen)?;
        let mut args = Vec::new();
        while self.peek() != Some(&Token::RParen) {
            let named = matches!(self.tokens.get(self.pos), Some(Token::Ident(_)))
                && matches!(self.tokens.get(self.pos + 1), Some(Token::Colon));
            if named {
                let k = self.expect_ident()?;
                self.expect(&Token::Colon)?;
                args.push((Some(k), self.parse_expr()?));
            } else {
                args.push((None, self.parse_expr()?));
            }
            if self.peek() == Some(&Token::Comma) { self.advance(); }
        }
        self.expect(&Token::RParen)?;
        Ok(args)
    }
}

pub fn compile_nx_source(source: &str) -> Result<(String, String), String> {
    let tokens: Vec<Token> = Token::lexer(source)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "Error léxico en archivo .nx".to_string())?;
    let mut parser = NxParser::new(tokens);
    let prog = parser.parse_program()?;

    let mut rust_out = String::from(
        "use nx_core::{Vec2, Vec3, Vec4, Mat4, Quat, FrameArena};
         use nx_ecs::*;
         use nx_hal_sensors::{GLOBAL_IMU, trigger_haptic_pulse};

",
    );
    let wgsl_out = String::from("// WGSL Generated by NativX
");

    for s in &prog.structs {
        rust_out.push_str(&format!(
            "#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct {} {{
",
            s.name
        ));
        for (fnm, fty) in &s.fields {
            rust_out.push_str(&format!("    pub {}: {},
", fnm, type_to_rust(fty)));
        }
        rust_out.push_str("}

");
    }

    for f in &prog.functions {
        let mut params = vec!["frame_arena: &mut FrameArena".to_string()];
        for (pn, pt) in &f.params {
            params.push(format!("{}: {}", pn, type_to_rust(pt)));
        }
        let ret = if f.return_type == NxType::Void { String::new() } else { format!(" -> {}", type_to_rust(&f.return_type)) };
        rust_out.push_str(&format!("pub fn {}({}){} {{
", f.name, params.join(", "), ret));
        for st in &f.body {
            rust_out.push_str(&format!("    {}
", stmt_to_rust(st)));
        }
        rust_out.push_str("}

");
    }

    Ok((rust_out, wgsl_out))
}

fn type_to_rust(t: &NxType) -> String {
    match t {
        NxType::F32 => "f32".into(), NxType::I32 => "i32".into(), NxType::U32 => "u32".into(),
        NxType::Bool => "bool".into(), NxType::Vec2 => "Vec2".into(), NxType::Vec3 => "Vec3".into(),
        NxType::Vec4 => "Vec4".into(), NxType::Mat4 => "Mat4".into(), NxType::Quat => "Quat".into(),
        NxType::World => "World".into(), NxType::EntityId => "EntityId".into(),
        NxType::CommandBuffer => "CommandBuffer".into(),
        NxType::Array(inner, n) => format!("[{}; {}]", type_to_rust(inner), n),
        NxType::Custom(s) => s.clone(),
        NxType::Ref(i) => format!("&{}", type_to_rust(i)),
        NxType::MutRef(i) => format!("&mut {}", type_to_rust(i)),
        _ => "()".into(),
    }
}

fn stmt_to_rust(st: &Stmt) -> String {
    match st {
        Stmt::VarDecl { kind, name, value, .. } => match kind {
            VarKind::Temp => format!("let {} = *frame_arena.alloc({});", name, expr_to_rust(value)),
            VarKind::Var => format!("let mut {} = {};", name, expr_to_rust(value)),
            _ => format!("let {} = {};", name, expr_to_rust(value)),
        },
        Stmt::IfStmt { condition, then_branch, else_branch } => {
            let tb: Vec<String> = then_branch.iter().map(stmt_to_rust).collect();
            let mut s = format!("if {} {{
        {}
    }}", expr_to_rust(condition), tb.join("
        "));
            if let Some(eb) = else_branch {
                let ebs: Vec<String> = eb.iter().map(stmt_to_rust).collect();
                s.push_str(&format!(" else {{
        {}
    }}", ebs.join("
        ")));
            }
            s
        }
        Stmt::ForRangeStmt { iterator, start, end, body } => {
            let b: Vec<String> = body.iter().map(stmt_to_rust).collect();
            format!("for {} in ({})..({}) {{
        {}
    }}", iterator, expr_to_rust(start), expr_to_rust(end), b.join("
        "))
        }
        Stmt::ForEcsQueryStmt { world_expr, query_mode, entity_var, component_vars, component_types, body } => {
            let w = expr_to_rust(world_expr);
            let b: Vec<String> = body.iter().map(stmt_to_rust).collect();
            let t1 = type_to_rust(&component_types[0]);
            let t2 = type_to_rust(&component_types[1]);
            let m = if *query_mode == EcsQueryMode::PairMut { "query_pair_mut" } else { "query_mut_ref" };
            format!("{}.{}::<{}, {}, _>(|{}, {}, {}| {{
        {}
    }});", w, m, t1, t2, entity_var, component_vars[0], component_vars[1], b.join("
        "))
        }
        Stmt::ReturnStmt(Some(e)) => format!("return {};", expr_to_rust(e)),
        Stmt::ReturnStmt(None) => "return;".into(),
        Stmt::ExprStmt(e) => format!("{};", expr_to_rust(e)),
        _ => String::new(),
    }
}

fn expr_to_rust(e: &Expr) -> String {
    match e {
        Expr::FloatLit(f) => format!("{:.4}_f32", f),
        Expr::IntLit(i) => format!("{}", i),
        Expr::Ident(s) => s.clone(),
        Expr::ArrayLit(v) => {
            let els: Vec<String> = v.iter().map(expr_to_rust).collect();
            format!("[{}]", els.join(", "))
        }
        Expr::IndexAccess { array, index } => format!("{}[({}) as usize]", expr_to_rust(array), expr_to_rust(index)),
        Expr::FieldAccess { base, field } => {
            let b = expr_to_rust(base);
            if matches!(field.as_str(), "position" | "rotation" | "scale") && b == "tr" {
                format!("tr.0.{}", field)
            } else {
                format!("{}.{}", b, field)
            }
        }
        Expr::MethodCall { receiver, method, args } => {
            let r = expr_to_rust(receiver);
            let a: Vec<String> = args.iter().map(|(_, x)| expr_to_rust(x)).collect();
            if r.contains("imu") && method == "gyroscope" {
                return "Vec3::from_array(GLOBAL_IMU.read_gyro())".into();
            }
            if matches!(r.as_str(), "LocalTransform" | "RigidBody3D" | "ColliderAabb") {
                return format!("{}::{}({})", r, method, a.join(", "));
            }
            if method == "step_physics" {
                return format!("{{ system_integrate_physics({}, {}, {}); system_resolve_aabb_collisions({}, frame_arena); }}", r, a[0], a[1], r);
            }
            if method == "propagate_transforms" {
                return format!("system_propagate_transforms({})", r);
            }
            format!("{}.{}({})", r, method, a.join(", "))
        }
        Expr::FnCall { name, args } => {
            let a: Vec<String> = args.iter().map(|(_, x)| expr_to_rust(x)).collect();
            match name.as_str() {
                "vec2" => format!("Vec2::new({})", a.join(", ")),
                "vec3" => format!("Vec3::new({})", a.join(", ")),
                "vec4" => format!("Vec4::new({})", a.join(", ")),
                _ => format!("{}({})", name, a.join(", ")),
            }
        }
        Expr::StructInit { name, fields } => {
            let mut fs: Vec<String> = fields.iter().map(|(k, v)| format!("{}: {}", k, expr_to_rust(v))).collect();
            fs.push("..Default::default()".into());
            format!("{} {{ {} }}", name, fs.join(", "))
        }
        Expr::Unary { op: UnaryOp::Neg, expr } => format!("(-{})", expr_to_rust(expr)),
        Expr::Unary { op: UnaryOp::Not, expr } => format!("(!{})", expr_to_rust(expr)),
        Expr::Binary { left, op, right } => {
            let s = match op {
                BinOp::Add => "+", BinOp::Sub => "-", BinOp::Mul => "*", BinOp::Div => "/",
                BinOp::Eq => "==", BinOp::Less => "<", BinOp::Greater => ">",
                BinOp::Assign => "=", BinOp::AddAssign => "+=", BinOp::SubAssign => "-=", _ => "*=",
            };
            format!("{} {} {}", expr_to_rust(left), s, expr_to_rust(right))
        }
        _ => String::new(),
    }
}
